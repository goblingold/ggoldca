import * as wh from "@orca-so/whirlpools-sdk";
import {
  AnchorProvider,
  BN,
  Idl,
  Program,
  utils,
  web3,
} from "@project-serum/anchor";
import { MintLayout } from "@solana/spl-token";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
} from "@solana/spl-token2";
import { Decimal } from "decimal.js";
import { Fetcher } from "./fetcher";
import IDL from "./idl/ggoldca.json";

const DAO_TREASURY_PUBKEY = new web3.PublicKey(
  "8XhNoDjjNoLP5Rys1pBJKGdE8acEC1HJsWGkfkMt6JP1"
);

interface InitializeVaultParams {
  userSigner: web3.PublicKey;
  poolId: web3.PublicKey;
}

interface OpenPositionParams {
  lowerPrice: Decimal;
  upperPrice: Decimal;
  userSigner: web3.PublicKey;
  poolId: web3.PublicKey;
  positionMint: web3.PublicKey;
}

interface DepositParams {
  lpAmount: BN;
  maxAmountA: BN;
  maxAmountB: BN;
  userSigner: web3.PublicKey;
  poolId: web3.PublicKey;
  position: PositionAccounts;
}

interface WithdrawParams {
  lpAmount: BN;
  minAmountA: BN;
  minAmountB: BN;
  userSigner: web3.PublicKey;
  poolId: web3.PublicKey;
  position: PositionAccounts;
}

interface DepositWithdrawAccounts {
  userSigner: web3.PublicKey;
  vaultAccount: web3.PublicKey;
  vaultLpTokenMintPubkey: web3.PublicKey;
  vaultInputTokenAAccount: web3.PublicKey;
  vaultInputTokenBAccount: web3.PublicKey;
  userLpTokenAccount: web3.PublicKey;
  userTokenAAccount: web3.PublicKey;
  userTokenBAccount: web3.PublicKey;
  whirlpoolProgramId: web3.PublicKey;
  position: PositionAccounts;
  whTokenVaultA: web3.PublicKey;
  whTokenVaultB: web3.PublicKey;
  tokenProgram: web3.PublicKey;
}

interface PositionAccounts {
  whirlpool: web3.PublicKey;
  position: web3.PublicKey;
  positionTokenAccount: web3.PublicKey;
  tickArrayLower: web3.PublicKey;
  tickArrayUpper: web3.PublicKey;
}

interface ConstructorParams {
  programId: web3.PublicKey;
  connection: web3.Connection;
}

export class GGoldcaSDK {
  program;
  fetcher: Fetcher;
  connection: web3.Connection;

  public constructor(params: ConstructorParams) {
    const { programId, connection } = params;

    this.connection = connection;
    this.fetcher = new Fetcher(connection, programId);
    this.program = new Program(
      IDL as Idl,
      programId,
      null as unknown as AnchorProvider
    );
  }

  async initializeVaultTx(
    params: InitializeVaultParams
  ): Promise<web3.Transaction> {
    const { poolId, userSigner } = params;
    const {
      vaultAccount,
      vaultLpTokenMintPubkey,
      vaultInputTokenAAccount,
      vaultInputTokenBAccount,
    } = await this.fetcher.getVaultKeys(poolId);

    const poolData = await this.fetcher.getWhirlpoolData(poolId);

    const daoTreasuryLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      DAO_TREASURY_PUBKEY,
      false
    );

    const tx = await this.program.methods
      .initializeVault()
      .accounts({
        userSigner,
        whirlpool: poolId,
        inputTokenAMintAddress: poolData.tokenMintA,
        inputTokenBMintAddress: poolData.tokenMintB,
        vaultAccount,
        vaultLpTokenMintPubkey,
        vaultInputTokenAAccount,
        vaultInputTokenBAccount,
        daoTreasuryLpTokenAccount,
        daoTreasuryOwner: DAO_TREASURY_PUBKEY,
        systemProgram: web3.SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .transaction();

    // Create rewards ATAs
    const rewardMints = poolData.rewardInfos
      .map((info) => info.mint)
      .filter((k) => k.toString() !== web3.PublicKey.default.toString());

    const rewardAccounts = await Promise.all(
      rewardMints.map(async (key) =>
        getAssociatedTokenAddress(key, vaultAccount, true)
      )
    );

    rewardAccounts.forEach((pubkey, indx) => {
      tx.add(
        createAssociatedTokenAccountInstruction(
          userSigner,
          pubkey,
          vaultAccount,
          rewardMints[indx]
        )
      );
    });

    return tx;
  }

  async openPositionIx(
    params: OpenPositionParams
  ): Promise<web3.TransactionInstruction> {
    const { lowerPrice, upperPrice, userSigner, poolId, positionMint } = params;

    const poolData = await this.fetcher.getWhirlpoolData(poolId);
    const fetcher = new wh.AccountFetcher(this.connection);

    const [tokenMintAInfo, tokenMintBInfo] =
      await utils.rpc.getMultipleAccounts(this.connection, [
        poolData.tokenMintA,
        poolData.tokenMintB,
      ]);

    if (!tokenMintAInfo) {
      throw new Error("Cannot fetch" + poolData.tokenMintA.toString());
    }
    if (!tokenMintBInfo) {
      throw new Error("Cannot fetch" + poolData.tokenMintB.toString());
    }

    const mintA = MintLayout.decode(tokenMintAInfo.account.data);
    const mintB = MintLayout.decode(tokenMintBInfo.account.data);

    const tokenADecimal = mintA.decimals;
    const tokenBDecimal = mintB.decimals;

    const { vaultAccount } = await this.fetcher.getVaultKeys(poolId);

    const tickLower = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(lowerPrice, tokenADecimal, tokenBDecimal),
      poolData.tickSpacing
    );
    const tickUpper = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(upperPrice, tokenADecimal, tokenBDecimal),
      poolData.tickSpacing
    );

    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMint
    );

    const positionTokenAccount = await getAssociatedTokenAddress(
      positionMint,
      vaultAccount,
      true
    );

    return this.program.methods
      .openPosition(positionPda.bump, tickLower, tickUpper)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position: positionPda.publicKey,
        positionMint,
        positionTokenAccount,
        whirlpool: poolId,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .instruction();
  }

  async depositIx(params: DepositParams): Promise<web3.TransactionInstruction> {
    const { lpAmount, maxAmountA, maxAmountB, userSigner, poolId, position } =
      params;

    const accounts = await this.depositWithdrawAccounts(
      userSigner,
      poolId,
      position
    );

    return this.program.methods
      .deposit(lpAmount, maxAmountA, maxAmountB)
      .accounts(accounts)
      .instruction();
  }

  async withdrawIx(
    params: WithdrawParams
  ): Promise<web3.TransactionInstruction> {
    const { lpAmount, minAmountA, minAmountB, userSigner, poolId, position } =
      params;

    const accounts = await this.depositWithdrawAccounts(
      userSigner,
      poolId,
      position
    );

    return this.program.methods
      .withdraw(lpAmount, minAmountA, minAmountB)
      .accounts(accounts)
      .instruction();
  }

  async depositWithdrawAccounts(
    userSigner,
    poolId,
    position
  ): Promise<DepositWithdrawAccounts> {
    const poolData = await this.fetcher.getWhirlpoolData(poolId);

    const {
      vaultAccount,
      vaultLpTokenMintPubkey,
      vaultInputTokenAAccount,
      vaultInputTokenBAccount,
    } = await this.fetcher.getVaultKeys(poolId);

    const [userLpTokenAccount, userTokenAAccount, userTokenBAccount] =
      await Promise.all(
        [vaultLpTokenMintPubkey, poolData.tokenMintA, poolData.tokenMintB].map(
          async (key) => getAssociatedTokenAddress(key, userSigner)
        )
      );

    return {
      userSigner,
      vaultAccount,
      vaultLpTokenMintPubkey,
      vaultInputTokenAAccount,
      vaultInputTokenBAccount,
      userLpTokenAccount,
      userTokenAAccount,
      userTokenBAccount,
      whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      position,
      whTokenVaultA: poolData.tokenVaultA,
      whTokenVaultB: poolData.tokenVaultB,
      tokenProgram: TOKEN_PROGRAM_ID,
    };
  }
}

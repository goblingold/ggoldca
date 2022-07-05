import * as wh from "@orca-so/whirlpools-sdk";
import { AnchorProvider, BN, Idl, Program, web3 } from "@project-serum/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
} from "@solana/spl-token2";
import IDL from "./idl/ggoldca.json";

const DAO_TREASURY_PUBKEY = new web3.PublicKey(
  "8XhNoDjjNoLP5Rys1pBJKGdE8acEC1HJsWGkfkMt6JP1"
);

interface InitializeVaultParams {
  userSigner: web3.PublicKey;
  poolId: web3.PublicKey;
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

interface VaultKeys {
  vaultAccount: web3.PublicKey;
  vaultLpTokenMintPubkey: web3.PublicKey;
  vaultInputTokenAAccount: web3.PublicKey;
  vaultInputTokenBAccount: web3.PublicKey;
}

interface CachedData {
  whirlpool: Record<string, wh.WhirlpoolData>;
  vaultKeys: Record<string, VaultKeys>;
}

interface ConstructorParams {
  programId: web3.PublicKey;
  connection: web3.Connection;
}

export class GGoldcaSDK {
  program;
  connection;

  cached: CachedData = { whirlpool: {}, vaultKeys: {} };

  public constructor(params: ConstructorParams) {
    this.connection = params.connection;
    this.program = new Program(
      IDL as Idl,
      params.programId,
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
    } = await this.getVaultKeys(poolId);

    const poolData = await this.getWhirlpoolData(poolId);

    const daoTreasuryLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      DAO_TREASURY_PUBKEY,
      false
    );

    const tx = await this.program.methods
      .initializeVault()
      .accounts({
        userSigner,
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

  async getVaultKeys(poolId: web3.PublicKey): Promise<VaultKeys> {
    const poolData = await this.getWhirlpoolData(poolId);

    const [vaultAccount, _bumpVault] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        poolData.tokenMintA.toBuffer(),
        poolData.tokenMintB.toBuffer(),
      ],
      this.program.programId
    );

    const [vaultLpTokenMintPubkey, _bumpLp] =
      web3.PublicKey.findProgramAddressSync(
        [Buffer.from("mint"), vaultAccount.toBuffer()],
        this.program.programId
      );

    const [vaultInputTokenAAccount, vaultInputTokenBAccount] =
      await Promise.all(
        [poolData.tokenMintA, poolData.tokenMintB].map(async (key) =>
          getAssociatedTokenAddress(key, vaultAccount, true)
        )
      );

    return {
      vaultAccount,
      vaultLpTokenMintPubkey,
      vaultInputTokenAAccount,
      vaultInputTokenBAccount,
    };
  }

  async getWhirlpoolData(poolId: web3.PublicKey): Promise<wh.WhirlpoolData> {
    const key = poolId.toString();
    if (!this.cached.whirlpool[key]) {
      const fetcher = new wh.AccountFetcher(this.connection);
      const poolData = await fetcher.getPool(poolId);

      if (!poolData) throw new Error("Cannot fetch pool " + key);
      this.cached.whirlpool[key] = poolData;
    }
    return this.cached.whirlpool[key];
  }

  async depositWithdrawAccounts(
    userSigner,
    poolId,
    position
  ): Promise<DepositWithdrawAccounts> {
    const poolData = await this.getWhirlpoolData(poolId);

    const {
      vaultAccount,
      vaultLpTokenMintPubkey,
      vaultInputTokenAAccount,
      vaultInputTokenBAccount,
    } = await this.getVaultKeys(poolId);

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

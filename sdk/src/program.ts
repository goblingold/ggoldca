import * as wh from "@orca-so/whirlpools-sdk";
import { AnchorProvider, BN, Idl, Program, web3 } from "@project-serum/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
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
  accounts: DepositWithdrawAccounts;
}

interface WithdrawParams {
  lpAmount: BN;
  minAmountA: BN;
  minAmountB: BN;
  accounts: DepositWithdrawAccounts;
}

interface DepositWithdrawAccounts {
  accounts: {
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
  };
}

interface PositionAccounts {
  whirlpool: web3.PublicKey;
  position: web3.PublicKey;
  positionTokenAccount: web3.PublicKey;
  tickArrayLower: web3.PublicKey;
  tickArrayUpper: web3.PublicKey;
}

interface CachedData {
  whirlpool: Record<string, wh.WhirlpoolData>;
}

interface ConstructorParams {
  programId: web3.PublicKey;
  connection: web3.Connection;
}

export class GGoldcaSDK {
  program;
  connection;

  cached: CachedData = { whirlpool: {} };

  public constructor(params: ConstructorParams) {
    this.connection = params.connection;
    this.program = new Program(
      IDL as Idl,
      params.programId,
      null as unknown as AnchorProvider
    );
  }

  async initializeVaultIx(
    params: InitializeVaultParams
  ): Promise<web3.TransactionInstruction> {
    const { poolId, userSigner } = params;

    const _poolId = poolId.toString();
    if (!this.cached.whirlpool[_poolId]) {
      const fetcher = new wh.AccountFetcher(this.connection);
      const poolData = await fetcher.getPool(poolId);

      if (!poolData) {
        throw new Error("Cannot fetch pool " + poolId);
      }
      this.cached.whirlpool[_poolId] = poolData;
    }

    const poolData = this.cached.whirlpool[_poolId];

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

    const daoTreasuryLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      DAO_TREASURY_PUBKEY,
      false
    );

    return this.program.methods
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
      .instruction();
  }

  async depositIx(params: DepositParams): Promise<web3.TransactionInstruction> {
    const { lpAmount, maxAmountA, maxAmountB, accounts } = params;
    return this.program.methods
      .deposit(lpAmount, maxAmountA, maxAmountB)
      .accounts(accounts)
      .instruction();
  }

  async withdrawIx(
    params: WithdrawParams
  ): Promise<web3.TransactionInstruction> {
    const { lpAmount, minAmountA, minAmountB, accounts } = params;
    return this.program.methods
      .withdraw(lpAmount, minAmountA, minAmountB)
      .accounts(accounts)
      .instruction();
  }
}

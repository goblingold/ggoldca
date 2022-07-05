import { AnchorProvider, BN, Idl, Program, web3 } from "@project-serum/anchor";
import IDL from "./idl/ggoldca.json";

interface InitializeVaultParams {
  accounts: InitializeVaultAccounts;
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

interface InitializeVaultAccounts {
  userSigner: web3.PublicKey;
  inputTokenAMintAddress: web3.PublicKey;
  inputTokenBMintAddress: web3.PublicKey;
  vaultAccount: web3.PublicKey;
  vaultInputTokenAAccount: web3.PublicKey;
  vaultInputTokenBAccount: web3.PublicKey;
  vaultLpTokenMintPubkey: web3.PublicKey;
  daoTreasuryLpTokenAccount: web3.PublicKey;
  daoTreasuryOwner: web3.PublicKey;
  systemProgram: web3.PublicKey;
  associatedTokenProgram: web3.PublicKey;
  tokenProgram: web3.PublicKey;
  rent: web3.PublicKey;
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

export class GGoldcaSDK {
  program;

  public constructor(programId: web3.PublicKey) {
    this.program = new Program(
      IDL as Idl,
      programId,
      null as unknown as AnchorProvider
    );
  }

  async initializeVaultIx(
    params: InitializeVaultParams
  ): Promise<web3.TransactionInstruction> {
    const { accounts } = params;
    return this.program.methods
      .initializeVault()
      .accounts(accounts)
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

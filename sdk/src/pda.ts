import { web3 } from "@project-serum/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token2";
import { Fetcher } from "./fetcher";

interface VaultKeys {
  vaultAccount: web3.PublicKey;
  vaultLpTokenMintPubkey: web3.PublicKey;
  vaultInputTokenAAccount: web3.PublicKey;
  vaultInputTokenBAccount: web3.PublicKey;
}

export class PDAAccounts {
  fetcher: Fetcher;
  programId: web3.PublicKey;
  cached: Record<string, VaultKeys> = {};

  public constructor(fetcher: Fetcher, programId: web3.PublicKey) {
    this.fetcher = fetcher;
    this.programId = programId;
  }

  async getVaultKeys(poolId: web3.PublicKey): Promise<VaultKeys> {
    const key = poolId.toString();
    if (!this.cached[key]) {
      const [vaultAccount, _bumpVault] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), poolId.toBuffer()],
        this.programId
      );

      const [vaultLpTokenMintPubkey, _bumpLp] =
        web3.PublicKey.findProgramAddressSync(
          [Buffer.from("mint"), vaultAccount.toBuffer()],
          this.programId
        );

      const poolData = await this.fetcher.getWhirlpoolData(poolId);
      const [vaultInputTokenAAccount, vaultInputTokenBAccount] =
        await Promise.all(
          [poolData.tokenMintA, poolData.tokenMintB].map(async (key) =>
            getAssociatedTokenAddress(key, vaultAccount, true)
          )
        );

      this.cached[key] = {
        vaultAccount,
        vaultLpTokenMintPubkey,
        vaultInputTokenAAccount,
        vaultInputTokenBAccount,
      };
    }
    return this.cached[key];
  }
}

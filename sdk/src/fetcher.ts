import { AccountFetcher, WhirlpoolData } from "@orca-so/whirlpools-sdk";
import { web3 } from "@project-serum/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token2";

interface CachedData {
  whirlpool: Record<string, WhirlpoolData>;
  vaultKeys: Record<string, VaultKeys>;
}

interface VaultKeys {
  vaultAccount: web3.PublicKey;
  vaultLpTokenMintPubkey: web3.PublicKey;
  vaultInputTokenAAccount: web3.PublicKey;
  vaultInputTokenBAccount: web3.PublicKey;
}

export class Fetcher {
  connection: web3.Connection;
  programId: web3.PublicKey;

  cached: CachedData = {
    whirlpool: {},
    vaultKeys: {},
  };

  public constructor(connection: web3.Connection, programId: web3.PublicKey) {
    this.connection = connection;
    this.programId = programId;
  }

  async getVaultKeys(poolId: web3.PublicKey): Promise<VaultKeys> {
    const poolData = await this.getWhirlpoolData(poolId);

    const [vaultAccount, _bumpVault] = web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        poolData.tokenMintA.toBuffer(),
        poolData.tokenMintB.toBuffer(),
      ],
      this.programId
    );

    const [vaultLpTokenMintPubkey, _bumpLp] =
      web3.PublicKey.findProgramAddressSync(
        [Buffer.from("mint"), vaultAccount.toBuffer()],
        this.programId
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

  async getWhirlpoolData(poolId: web3.PublicKey): Promise<WhirlpoolData> {
    const key = poolId.toString();
    if (!this.cached.whirlpool[key]) {
      const fetcher = new AccountFetcher(this.connection);
      const poolData = await fetcher.getPool(poolId);

      if (!poolData) throw new Error("Cannot fetch pool " + key);
      this.cached.whirlpool[key] = poolData;
    }
    return this.cached.whirlpool[key];
  }
}

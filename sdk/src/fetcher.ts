import { AccountFetcher, WhirlpoolData } from "@orca-so/whirlpools-sdk";
import { web3 } from "@project-serum/anchor";

interface CachedData {
  whirlpool: Record<string, WhirlpoolData>;
}

export class Fetcher {
  connection: web3.Connection;

  cached: CachedData = {
    whirlpool: {},
  };

  public constructor(connection: web3.Connection) {
    this.connection = connection;
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

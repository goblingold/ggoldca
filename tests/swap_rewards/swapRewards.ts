import * as wh from "@orca-so/whirlpools-sdk";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import {
  createAssociatedTokenAccountInstruction,
  createTransferInstruction,
  getAssociatedTokenAddress,
} from "@solana/spl-token-v2";
import { assert } from "chai";
import { Decimal } from "decimal.js";
import { GGoldcaSDK, Pools, VaultId } from "ggoldca-sdk";
import { Ggoldca } from "../target/types/ggoldca";

const POOL_ID = new anchor.web3.PublicKey(Pools.USH_USDC);
const VAULT_ID = new anchor.BN(0);

const CONFIRM_OPTS: anchor.web3.ConfirmOptions = {
  skipPreflight: true,
};

describe("swapRewards", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Ggoldca as Program<Ggoldca>;
  const userSigner = program.provider.wallet.publicKey;

  const ggClient = new GGoldcaSDK({
    programId: program.programId,
    provider: program.provider,
    connection: program.provider.connection,
  });

  const vaultId: VaultId = {
    whirlpool: POOL_ID,
    id: VAULT_ID,
  };

  it("Initialize vault", async () => {
    const ixs = await ggClient.initializeVaultIxs({
      userSigner,
      vaultId,
      fee: new anchor.BN(10),
    });

    const tx = ixs.reduce(
      (tx, ix) => tx.add(ix),
      new anchor.web3.Transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("initialize_vault", txSig);
  });

  it("Set market rewards for swap", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(vaultId.whirlpool);

    const { vaultInputTokenBAccount } = await ggClient.pdaAccounts.getVaultKeys(
      vaultId
    );

    const rewardMints = poolData.rewardInfos
      .map((info) => info.mint)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    const marketData = [
      {
        id: { whirlpool: {} },
        minAmountOut: new anchor.BN(1),
      },
      {
        id: { orcaV2: {} },
        minAmountOut: new anchor.BN(1),
      },
    ];

    const tx = (
      await Promise.all(
        marketData.map((data, indx) =>
          ggClient.setMarketRewards({
            userSigner,
            vaultId,
            rewardsMint: rewardMints[indx],
            marketRewards: data,
            destinationTokenAccount: vaultInputTokenBAccount,
          })
        )
      )
    ).reduce((acc, ix) => acc.add(ix), new anchor.web3.Transaction());

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("set_market_rewards_swap", txSig);
  });

  it("Swap rewards", async () => {
    const [{ vaultAccount }, poolData] = await Promise.all([
      ggClient.pdaAccounts.getVaultKeys(vaultId),
      ggClient.fetcher.getWhirlpoolData(vaultId.whirlpool),
    ]);

    // transfer some lamports to simulate the collected rewards
    const rewardMints = poolData.rewardInfos
      .map((info) => info.mint)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    const userAtas = await Promise.all(
      rewardMints.map(async (key) =>
        getAssociatedTokenAddress(key, userSigner, false)
      )
    );

    const vaultRewardsAtas = await Promise.all(
      rewardMints.map(async (key) =>
        getAssociatedTokenAddress(key, vaultAccount, true)
      )
    );

    const ixsTransfer = vaultRewardsAtas.map((_, indx) =>
      createTransferInstruction(
        userAtas[indx],
        vaultRewardsAtas[indx],
        userSigner,
        1_000_000,
        []
      )
    );

    const ixs = await ggClient.swapRewardsIxs({ vaultId });

    const tx = [...ixsTransfer, ...ixs].reduce(
      (acc, ix) => acc.add(ix),
      new anchor.web3.Transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("swap_rewards", txSig);
  });

  it("", async () => {
    return new Promise((resolve) => setTimeout(resolve, 100));
  });
});

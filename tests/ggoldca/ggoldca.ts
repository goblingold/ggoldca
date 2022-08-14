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

const POOL_ID = new anchor.web3.PublicKey(Pools.USDH_USDC);
const VAULT_ID = new anchor.BN(0);

const CONFIRM_OPTS: anchor.web3.ConfirmOptions = {
  skipPreflight: true,
};

const COMPUTE_BUDGET_IX = new anchor.web3.TransactionInstruction({
  programId: new anchor.web3.PublicKey(
    "ComputeBudget111111111111111111111111111111"
  ),
  keys: [],
  data: Buffer.from(
    Uint8Array.of(0, ...new anchor.BN(1_000_000).toArray("le", 8))
  ),
});

describe("ggoldca", () => {
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

  let position;
  let position2;
  let positionMint;
  let position2Mint;

  it("Open position", async () => {
    const positionMintKeypair = anchor.web3.Keypair.generate();
    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMintKeypair.publicKey
    );

    position = positionPda.publicKey;
    positionMint = positionMintKeypair.publicKey;

    const ixs = await ggClient.openPositionIxs({
      lowerPrice: new Decimal(0.9),
      upperPrice: new Decimal(1.1),
      userSigner,
      vaultId,
      positionMint: positionMintKeypair.publicKey,
    });

    const tx = ixs.reduce(
      (tx, ix) => tx.add(ix),
      new anchor.web3.Transaction()
    );

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position", txSig);
  });

  it("Open position2", async () => {
    const positionMintKeypair = anchor.web3.Keypair.generate();
    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMintKeypair.publicKey
    );

    position2 = positionPda.publicKey;
    position2Mint = positionMintKeypair.publicKey;

    const ixs = await ggClient.openPositionIxs({
      lowerPrice: new Decimal(0.95),
      upperPrice: new Decimal(1.05),
      userSigner,
      vaultId,
      positionMint: positionMintKeypair.publicKey,
    });

    const tx = ixs.reduce(
      (tx, ix) => tx.add(ix),
      new anchor.web3.Transaction()
    );

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position_2", txSig);
  });

  it("Deposit", async () => {
    const lpAmount = new anchor.BN(1_000_000_000_000);
    const maxAmountA = new anchor.BN(1_000_000_000_000);
    const maxAmountB = new anchor.BN(1_000_000_000_000);

    const { vaultLpTokenMintPubkey } = await ggClient.pdaAccounts.getVaultKeys(
      vaultId
    );

    const userLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      userSigner
    );

    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(vaultId);

    const tx = new anchor.web3.Transaction()
      .add(
        createAssociatedTokenAccountInstruction(
          userSigner,
          userLpTokenAccount,
          userSigner,
          vaultLpTokenMintPubkey
        )
      )
      .add(
        await ggClient.depositIx({
          lpAmount,
          maxAmountA,
          maxAmountB,
          userSigner,
          vaultId,
        })
      );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit", txSig);
  });

  it("Deposit with tokens in vault", async () => {
    const lpAmount = new anchor.BN(2_000_000);
    const maxAmountA = new anchor.BN(1_000_000);
    const maxAmountB = new anchor.BN(1_000_000);

    const poolData = await ggClient.fetcher.getWhirlpoolData(POOL_ID);

    const userTokenAAccount = await getAssociatedTokenAddress(
      poolData.tokenMintA,
      userSigner
    );

    const { vaultInputTokenAAccount } = await ggClient.pdaAccounts.getVaultKeys(
      vaultId
    );

    const transferIx = createTransferInstruction(
      userTokenAAccount,
      vaultInputTokenAAccount,
      userSigner,
      1_000,
      []
    );

    const tx = new anchor.web3.Transaction().add(transferIx).add(
      await ggClient.depositIx({
        lpAmount,
        maxAmountA,
        maxAmountB,
        userSigner,
        vaultId,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit_with_tokens_in_vault", txSig);
  });

  it("Try collect fees", async () => {
    const ix = await ggClient.collectFeesIx({ vaultId, position });
    const tx = new anchor.web3.Transaction().add(ix);

    try {
      const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
      console.log("collect_fees", txSig);
    } catch (err) {
      const errNumber = program.idl.errors
        .filter((err) => err.name == "NotEnoughFees")
        .map((err) => err.code)[0];

      assert.include(err.toString(), errNumber);
      console.log("Not enought fees generated");
    }
  });

  it("Try collect rewards", async () => {
    const ixs = await ggClient.collectRewardsIxs({ vaultId, position });
    const tx = ixs.reduce(
      (tx, ix) => tx.add(ix),
      new anchor.web3.Transaction()
    );

    try {
      const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
      console.log("collect_rewards", txSig);
    } catch (err) {
      const errNumber = program.idl.errors
        .filter((err) => err.name == "NotEnoughRewards")
        .map((err) => err.code)[0];

      assert.include(err.toString(), errNumber);
      console.log("Not enought rewards generated");
    }
  });

  it("Reinvest", async () => {
    const [poolData, { vaultInputTokenAAccount, vaultInputTokenBAccount }] =
      await Promise.all([
        ggClient.fetcher.getWhirlpoolData(POOL_ID),
        ggClient.pdaAccounts.getVaultKeys(vaultId),
      ]);

    // transfer some lamports to simulate the collected fees
    const [userTokenAAccount, userTokenBAccount] = await Promise.all([
      getAssociatedTokenAddress(poolData.tokenMintA, userSigner),
      getAssociatedTokenAddress(poolData.tokenMintB, userSigner),
    ]);

    const transferAIx = createTransferInstruction(
      userTokenAAccount,
      vaultInputTokenAAccount,
      userSigner,
      1_000,
      []
    );

    const transferBIx = createTransferInstruction(
      userTokenBAccount,
      vaultInputTokenBAccount,
      userSigner,
      10_500,
      []
    );

    const tx = new anchor.web3.Transaction()
      .add(COMPUTE_BUDGET_IX)
      .add(transferAIx)
      .add(transferBIx)
      .add(await ggClient.reinvestIx({ vaultId }));

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("Reinvest", txSig);
  });

  it("Rebalance & reinvest", async () => {
    const [currentPosition, newPosition] = await Promise.all(
      [position, position2].map((key) =>
        ggClient.pdaAccounts.getPositionAccounts(key, vaultId)
      )
    );

    const [
      poolData,
      {
        vaultAccount,
        vaultLpTokenMintPubkey,
        vaultInputTokenAAccount,
        vaultInputTokenBAccount,
      },
    ] = await Promise.all([
      ggClient.fetcher.getWhirlpoolData(POOL_ID),
      ggClient.pdaAccounts.getVaultKeys(vaultId),
    ]);

    const oracleKeypair = wh.PDAUtil.getOracle(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID
    );

    const tickArrayAddresses = wh.PoolUtil.getTickArrayPublicKeysForSwap(
      poolData.tickCurrentIndex,
      poolData.tickSpacing,
      true,
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      newPosition.whirlpool
    );

    const tx = new anchor.web3.Transaction()
      .add(COMPUTE_BUDGET_IX)
      .add(
        await program.methods
          .rebalance()
          .accounts({
            userSigner,
            vaultAccount,
            vaultInputTokenAAccount,
            vaultInputTokenBAccount,
            whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
            tokenVaultA: poolData.tokenVaultA,
            tokenVaultB: poolData.tokenVaultB,
            currentPosition,
            newPosition,
          })
          .transaction()
      )
      .add(
        await program.methods
          .reinvest()
          .accounts({
            vaultAccount,
            vaultLpTokenMintPubkey,
            vaultInputTokenAAccount,
            vaultInputTokenBAccount,
            whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
            tokenVaultA: poolData.tokenVaultA,
            tokenVaultB: poolData.tokenVaultB,
            position: newPosition,
            tickArray0: tickArrayAddresses[0],
            tickArray1: tickArrayAddresses[1],
            tickArray2: tickArrayAddresses[2],
            oracle: oracleKeypair.publicKey,
          })
          .transaction()
      );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("Reinvest", txSig);
  });

  it("Failing swap in non-set market", async () => {
    const ixs = await ggClient.swapRewardsIxs({ vaultId });
    const tx = new anchor.web3.Transaction().add(ixs[0]);

    try {
      const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
      assert(false);
    } catch (err) {
      const errNumber = program.idl.errors
        .filter((err) => err.name == "SwapNotSet")
        .map((err) => err.code)[0];

      assert.include(err.toString(), errNumber);
      console.log("Unset swap market");
    }
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
        id: { orcaV2: {} },
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
        1_000,
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

  it("Set market rewards for transfer", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(vaultId.whirlpool);

    const rewardMints = poolData.rewardInfos
      .map((info) => info.mint)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    const userAtas = await Promise.all(
      rewardMints.map(async (key) =>
        getAssociatedTokenAddress(key, userSigner, false)
      )
    );

    const marketData = [
      {
        id: { transfer: {} },
        minAmountOut: new anchor.BN(1),
      },
      {
        id: { transfer: {} },
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
            destinationTokenAccount: userAtas[indx],
          })
        )
      )
    ).reduce((acc, ix) => acc.add(ix), new anchor.web3.Transaction());

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("set_market_rewards_transfer", txSig);
  });

  it("Transfer rewards", async () => {
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
        1_000,
        []
      )
    );

    const ixs = await Promise.all(
      rewardMints.map((_, indx) =>
        ggClient.transferRewards({
          vaultId,
          rewardsIndex: indx,
        })
      )
    );

    const tx = [...ixsTransfer, ...ixs].reduce(
      (acc, ix) => acc.add(ix),
      new anchor.web3.Transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("transfer_rewards", txSig);
  });

  it("Close position", async () => {
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(vaultId);

    // Try claim pending fees/rewards
    const ixFees = await ggClient.collectFeesIx({
      position,
      vaultId,
    });

    const ixRewards = await ggClient.collectRewardsIxs({
      position,
      vaultId,
    });

    const txs = [ixFees, ...ixRewards].map((ix) =>
      new anchor.web3.Transaction().add(ix)
    );

    const txSigs = await Promise.allSettled(
      txs.map(async (tx) =>
        program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS)
      )
    );

    const positionTokenAccount = await getAssociatedTokenAddress(
      positionMint,
      vaultAccount,
      true
    );

    const tx = await program.methods
      .closePosition()
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position: position,
        positionMint: positionMint,
        positionTokenAccount,
      })
      .transaction();

    const numPositionsBefore = (
      await program.account.vaultAccount.fetch(vaultAccount)
    ).positions.length;

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);

    const numPositionsAfter = (
      await program.account.vaultAccount.fetch(vaultAccount)
    ).positions.length;

    assert.equal(numPositionsAfter, numPositionsBefore - 1);
    console.log("close_position", txSig);
  });

  it("Failing closing position in use", async () => {
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(vaultId);

    const positionTokenAccount = await getAssociatedTokenAddress(
      position2Mint,
      vaultAccount,
      true
    );

    const tx = await program.methods
      .closePosition()
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position: position2,
        positionMint: position2Mint,
        positionTokenAccount,
      })
      .transaction();

    try {
      const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
      assert.ok(false);
    } catch (err) {
      assert.include(err.toString(), "6005");
    }
  });

  it("Withdraw", async () => {
    const lpAmount = new anchor.BN(3_000_000);
    const minAmountA = new anchor.BN(0);
    const minAmountB = new anchor.BN(0);

    const tx = new anchor.web3.Transaction().add(
      await ggClient.withdrawIx({
        lpAmount,
        minAmountA,
        minAmountB,
        userSigner,
        vaultId,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("withdraw", txSig);
  });

  it("set vault_account fee", async () => {
    const fee = new anchor.BN(50);
    const tx = new anchor.web3.Transaction().add(
      await ggClient.setVaultFee({
        userSigner,
        vaultId,
        fee,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("set fee", txSig);
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(vaultId);
    const data = await program.account.vaultAccount.fetch(vaultAccount);
    assert.ok(data.fee.toString() === fee.toString());
  });

  it("vault_account", async () => {
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(vaultId);
    const data = await program.account.vaultAccount.fetch(vaultAccount);
    console.log(JSON.stringify(data, null, 4));
    return new Promise((resolve) => setTimeout(resolve, 100));
  });
});

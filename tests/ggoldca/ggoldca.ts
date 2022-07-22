import * as wh from "@orca-so/whirlpools-sdk";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import {
  createAssociatedTokenAccountInstruction,
  createTransferInstruction,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { Decimal } from "decimal.js";
import { GGoldcaSDK } from "ggoldca-sdk";
import { Ggoldca } from "../target/types/ggoldca";

const POOL_ID = new anchor.web3.PublicKey(
  "Fvtf8VCjnkqbETA6KtyHYqHm26ut6w184Jqm4MQjPvv7"
);

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

  it("Initialize vault", async () => {
    const ixs = await ggClient.initializeVaultIxs({
      userSigner,
      poolId: POOL_ID,
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

  it("Open position", async () => {
    const positionMintKeypair = anchor.web3.Keypair.generate();
    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMintKeypair.publicKey
    );

    position = positionPda.publicKey;

    const ixs = await ggClient.openPositionIxs({
      lowerPrice: new Decimal(0.9),
      upperPrice: new Decimal(1.1),
      userSigner,
      poolId: POOL_ID,
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

    const ixs = await ggClient.openPositionIxs({
      lowerPrice: new Decimal(0.95),
      upperPrice: new Decimal(1.05),
      userSigner,
      poolId: POOL_ID,
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
      POOL_ID
    );

    const userLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      userSigner
    );

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
          poolId: POOL_ID,
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
      POOL_ID
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
        poolId: POOL_ID,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit_with_tokens_in_vault", txSig);
  });

  it("Collect fees", async () => {
    const ix = await ggClient.collectFeesIx({ userSigner, position });
    const tx = new anchor.web3.Transaction().add(ix);

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("collect_fees", txSig);
  });

  it("Collect rewards", async () => {
    const ixs = await ggClient.collectRewardsIxs({ userSigner, position });
    const tx = ixs.reduce(
      (tx, ix) => tx.add(ix),
      new anchor.web3.Transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("collect_rewards", txSig);
  });

  it("Rebalance", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(POOL_ID);

    const { vaultAccount, vaultInputTokenAAccount, vaultInputTokenBAccount } =
      await ggClient.pdaAccounts.getVaultKeys(POOL_ID);

    const [currentPosition, newPosition] = await Promise.all(
      [position, position2].map((key) =>
        ggClient.pdaAccounts.getPositionAccounts(key)
      )
    );

    const tx = new anchor.web3.Transaction().add(COMPUTE_BUDGET_IX).add(
      await program.methods
        .rebalance()
        .accounts({
          userSigner,
          vaultAccount,
          whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
          vaultInputTokenAAccount,
          vaultInputTokenBAccount,
          tokenVaultA: poolData.tokenVaultA,
          tokenVaultB: poolData.tokenVaultB,
          currentPosition,
          newPosition,
        })
        .transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("rebalance", txSig);
  });

  it("Reinvest", async () => {
    const [
      poolData,
      positionAccounts,
      { vaultAccount, vaultInputTokenAAccount, vaultInputTokenBAccount },
    ] = await Promise.all([
      ggClient.fetcher.getWhirlpoolData(POOL_ID),
      ggClient.pdaAccounts.getPositionAccounts(position2),
      ggClient.pdaAccounts.getVaultKeys(POOL_ID),
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
      positionAccounts.whirlpool
    );

    // transfer some lamports to simulate the collected rewards
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
      .add(
        await program.methods
          .reinvest()
          .accounts({
            userSigner,
            vaultAccount,
            whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
            vaultInputTokenAAccount,
            vaultInputTokenBAccount,
            tokenVaultA: poolData.tokenVaultA,
            tokenVaultB: poolData.tokenVaultB,
            position: positionAccounts,
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
        poolId: POOL_ID,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("withdraw", txSig);
  });

  it("vault_account", async () => {
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(POOL_ID);
    const data = await program.account.vaultAccount.fetch(vaultAccount);
    console.log(JSON.stringify(data, null, 4));
    return new Promise((resolve) => setTimeout(resolve, 100));
  });
});

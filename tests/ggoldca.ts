//#! anchor test
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
    connection: program.provider.connection,
    programId: program.programId,
  });

  const whFetcher = new wh.AccountFetcher(program.provider.connection);
  const whClient = wh.buildWhirlpoolClient(
    wh.WhirlpoolContext.withProvider(
      program.provider,
      wh.ORCA_WHIRLPOOL_PROGRAM_ID
    ),
    whFetcher
  );

  it("Initialize vault", async () => {
    const tx = await ggClient.initializeVaultTx({
      userSigner,
      poolId: POOL_ID,
    });
    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("initialize_vault", txSig);
  });

  let position;
  let positionTokenAccount;

  let position2;
  let position2TokenAccount;

  let positionAccounts;
  let positionAccounts2;

  it("Open position", async () => {
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(POOL_ID);

    const positionMintKeypair = anchor.web3.Keypair.generate();
    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMintKeypair.publicKey
    );
    const positionTokenAccountAddress = await getAssociatedTokenAddress(
      positionMintKeypair.publicKey,
      vaultAccount,
      true
    );

    position = positionPda.publicKey;
    positionTokenAccount = positionTokenAccountAddress;

    const tx = new anchor.web3.Transaction().add(
      await ggClient.openPositionIx({
        lowerPrice: new Decimal(0.9),
        upperPrice: new Decimal(1.1),
        userSigner,
        poolId: POOL_ID,
        positionMint: positionMintKeypair.publicKey,
      })
    );

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position", txSig);
  });

  it("Open position2", async () => {
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(POOL_ID);

    const positionMintKeypair = anchor.web3.Keypair.generate();
    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMintKeypair.publicKey
    );
    const positionTokenAccountAddress = await getAssociatedTokenAddress(
      positionMintKeypair.publicKey,
      vaultAccount,
      true
    );

    position2 = positionPda.publicKey;
    position2TokenAccount = positionTokenAccountAddress;

    const tx = new anchor.web3.Transaction().add(
      await ggClient.openPositionIx({
        lowerPrice: new Decimal(0.95),
        upperPrice: new Decimal(1.05),
        userSigner,
        poolId: POOL_ID,
        positionMint: positionMintKeypair.publicKey,
      })
    );

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position_2", txSig);
  });

  it("Init tick arrays", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(POOL_ID);
    const positionData = await whFetcher.getPosition(position);

    const startTickLower = wh.TickUtil.getStartTickIndex(
      positionData.tickLowerIndex,
      poolData.tickSpacing
    );

    const startTickUpper = wh.TickUtil.getStartTickIndex(
      positionData.tickUpperIndex,
      poolData.tickSpacing
    );

    const tickArrayLowerPda = wh.PDAUtil.getTickArray(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID,
      startTickLower
    );

    const tickArrayUpperPda = wh.PDAUtil.getTickArray(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID,
      startTickUpper
    );

    const initTickLowerIx = wh.WhirlpoolIx.initTickArrayIx(
      whClient.ctx.program,
      {
        startTick: startTickLower,
        tickArrayPda: tickArrayLowerPda,
        whirlpool: POOL_ID,
        funder: userSigner,
      }
    );

    const initTickUpperIx = wh.WhirlpoolIx.initTickArrayIx(
      whClient.ctx.program,
      {
        startTick: startTickUpper,
        tickArrayPda: tickArrayUpperPda,
        whirlpool: POOL_ID,
        funder: userSigner,
      }
    );

    positionAccounts = {
      whirlpool: POOL_ID,
      position,
      positionTokenAccount,
      tickArrayLower: tickArrayLowerPda.publicKey,
      tickArrayUpper: tickArrayUpperPda.publicKey,
    };

    const tx = new anchor.web3.Transaction()
      .add(initTickLowerIx)
      .add(initTickUpperIx);

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("init_tick_arrays", txSig);
  });

  it("Init tick arrays 2", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(POOL_ID);
    const positionData = await whFetcher.getPosition(position2);

    const startTickLower = wh.TickUtil.getStartTickIndex(
      positionData.tickLowerIndex,
      poolData.tickSpacing
    );

    const startTickUpper = wh.TickUtil.getStartTickIndex(
      positionData.tickUpperIndex,
      poolData.tickSpacing
    );

    const tickArrayLowerPda = wh.PDAUtil.getTickArray(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID,
      startTickLower
    );

    const tickArrayUpperPda = wh.PDAUtil.getTickArray(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID,
      startTickUpper
    );

    const initTickLowerIx = wh.WhirlpoolIx.initTickArrayIx(
      whClient.ctx.program,
      {
        startTick: startTickLower,
        tickArrayPda: tickArrayLowerPda,
        whirlpool: POOL_ID,
        funder: userSigner,
      }
    );

    const initTickUpperIx = wh.WhirlpoolIx.initTickArrayIx(
      whClient.ctx.program,
      {
        startTick: startTickUpper,
        tickArrayPda: tickArrayUpperPda,
        whirlpool: POOL_ID,
        funder: userSigner,
      }
    );

    positionAccounts2 = {
      whirlpool: POOL_ID,
      position: position2,
      positionTokenAccount: position2TokenAccount,
      tickArrayLower: tickArrayLowerPda.publicKey,
      tickArrayUpper: tickArrayUpperPda.publicKey,
    };

    const tx = new anchor.web3.Transaction()
      .add(initTickLowerIx)
      .add(initTickUpperIx);

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("init_tick_arrays_2", txSig);
  });

  it("Deposit", async () => {
    const lpAmount = new anchor.BN(1_000_000);
    const maxAmountA = new anchor.BN(1_000_000);
    const maxAmountB = new anchor.BN(1_000_000);

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
          position: positionAccounts,
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
        position: positionAccounts,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit_with_tokens_in_vault", txSig);
  });

  it("Collect fees & rewards", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(POOL_ID);
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(POOL_ID);

    const rewardWhirlpoolVaults = poolData.rewardInfos
      .map((info) => info.vault)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    const rewardMints = poolData.rewardInfos
      .map((info) => info.mint)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    const rewardAccounts = await Promise.all(
      rewardMints.map(async (key) =>
        getAssociatedTokenAddress(key, vaultAccount, true)
      )
    );

    const [tokenOwnerAccountA, tokenOwnerAccountB] = await Promise.all(
      [poolData.tokenMintA, poolData.tokenMintB].map((key) =>
        getAssociatedTokenAddress(key, userSigner)
      )
    );

    const remainingAccounts = [...rewardAccounts, ...rewardWhirlpoolVaults].map(
      (pubkey) =>
        (anchor.web3.AccountMeta = {
          isSigner: false,
          isWritable: true,
          pubkey,
        })
    );

    const tx = await program.methods
      .collectFeesAndRewards()
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        tokenOwnerAccountA,
        tokenOwnerAccountB,
        tokenVaultA: poolData.tokenVaultA,
        tokenVaultB: poolData.tokenVaultB,
        position: positionAccounts,
      })
      .remainingAccounts(remainingAccounts)
      .transaction();

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("collect_fees_and_rewards", txSig);
  });

  it("Rebalance", async () => {
    const poolData = await ggClient.fetcher.getWhirlpoolData(POOL_ID);
    const { vaultAccount } = await ggClient.pdaAccounts.getVaultKeys(POOL_ID);

    const [tokenOwnerAccountA, tokenOwnerAccountB] = await Promise.all(
      [poolData.tokenMintA, poolData.tokenMintB].map((key) =>
        getAssociatedTokenAddress(key, userSigner)
      )
    );

    const { vaultInputTokenAAccount, vaultInputTokenBAccount } =
      await ggClient.pdaAccounts.getVaultKeys(POOL_ID);

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
          currentPosition: positionAccounts,
          newPosition: positionAccounts2,
        })
        .transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("rebalance", txSig);
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
        position: positionAccounts2,
      })
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("withdraw", txSig);
  });

  it("Wait some time so validator logs are written", async () => {
    return new Promise((resolve) => setTimeout(resolve, 100));
  });
});

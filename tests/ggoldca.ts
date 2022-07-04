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
import { Ggoldca } from "../target/types/ggoldca";

const POOL_ID = new anchor.web3.PublicKey(
  "Fvtf8VCjnkqbETA6KtyHYqHm26ut6w184Jqm4MQjPvv7"
);

const TOKEN_A_MINT_PUBKEY = new anchor.web3.PublicKey(
  "USDH1SM1ojwWUga67PGrgFWUHibbjqMvuMaDkRJTgkX"
);

const TOKEN_B_MINT_PUBKEY = new anchor.web3.PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

const DAO_TREASURY_PUBKEY = new anchor.web3.PublicKey(
  "8XhNoDjjNoLP5Rys1pBJKGdE8acEC1HJsWGkfkMt6JP1"
);

const CONFIRM_OPTS: anchor.web3.ConfirmOptions = {
  skipPreflight: true,
};

const CONFIRM_OPTS_FIN: anchor.web3.ConfirmOptions = {
  skipPreflight: true,
  commitment: "finalized",
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

  const whClient = wh.buildWhirlpoolClient(
    wh.WhirlpoolContext.withProvider(
      program.provider,
      wh.ORCA_WHIRLPOOL_PROGRAM_ID
    ),
    new wh.AccountFetcher(program.provider.connection)
  );

  let vaultInputTokenAAccount;
  let vaultInputTokenBAccount;

  const [vaultAccount, bumpVault] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        TOKEN_A_MINT_PUBKEY.toBuffer(),
        TOKEN_B_MINT_PUBKEY.toBuffer(),
      ],
      program.programId
    );

  const [vaultLpTokenMintPubkey, bumpLp] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint"), vaultAccount.toBuffer()],
      program.programId
    );

  let rewardAccounts;
  let rewardWhirlpoolVaults;
  it("Initialize vault", async () => {
    const poolData = await whClient.fetcher.getPool(POOL_ID);
    const rewardMints = poolData.rewardInfos
      .map((info) => info.mint)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    rewardWhirlpoolVaults = poolData.rewardInfos
      .map((info) => info.vault)
      .filter((k) => k.toString() !== anchor.web3.PublicKey.default.toString());

    rewardAccounts = await Promise.all(
      rewardMints.map(async (key) =>
        getAssociatedTokenAddress(key, vaultAccount, true)
      )
    );

    vaultInputTokenAAccount = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      vaultAccount,
      true
    );

    vaultInputTokenBAccount = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      vaultAccount,
      true
    );

    const daoTreasuryLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      DAO_TREASURY_PUBKEY,
      false
    );

    const tx = await program.methods
      .initializeVault(bumpVault, bumpLp)
      .accounts({
        userSigner,
        inputTokenAMintAddress: TOKEN_A_MINT_PUBKEY,
        inputTokenBMintAddress: TOKEN_B_MINT_PUBKEY,
        vaultAccount,
        vaultInputTokenAAccount,
        vaultInputTokenBAccount,
        vaultLpTokenMintPubkey,
        daoTreasuryLpTokenAccount,
        daoTreasuryOwner: DAO_TREASURY_PUBKEY,
      })
      .transaction();

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
    const pool = await whClient.getPool(POOL_ID);
    const poolData = pool.getData();
    const poolTokenAInfo = pool.getTokenAInfo();
    const poolTokenBInfo = pool.getTokenBInfo();

    const tokenADecimal = poolTokenAInfo.decimals;
    const tokenBDecimal = poolTokenBInfo.decimals;

    const tickLower = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(0.9),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );
    const tickUpper = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(1.1),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );

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

    const tx = await program.methods
      .openPosition(positionPda.bump, tickLower, tickUpper)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position,
        positionMint: positionMintKeypair.publicKey,
        positionTokenAccount,
        whirlpool: POOL_ID,
      })
      .transaction();

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position", txSig);
  });

  it("Open position2", async () => {
    const pool = await whClient.getPool(POOL_ID);
    const poolData = pool.getData();
    const poolTokenAInfo = pool.getTokenAInfo();
    const poolTokenBInfo = pool.getTokenBInfo();

    const tokenADecimal = poolTokenAInfo.decimals;
    const tokenBDecimal = poolTokenBInfo.decimals;
    const tickLower = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(0.95),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );
    const tickUpper = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(1.05),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );

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

    const tx = await program.methods
      .openPosition(positionPda.bump, tickLower, tickUpper)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position: position2,
        positionMint: positionMintKeypair.publicKey,
        positionTokenAccount: position2TokenAccount,
        whirlpool: POOL_ID,
      })
      .transaction();

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position_2", txSig);
  });

  it("Init tick arrays", async () => {
    const positionData = await whClient.fetcher.getPosition(position);
    const poolData = await whClient.fetcher.getPool(POOL_ID);

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
    const positionData = await whClient.fetcher.getPosition(position2);
    const poolData = await whClient.fetcher.getPool(POOL_ID);

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
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    const lpAmount = new anchor.BN(1_000_000);
    const maxAmountA = new anchor.BN(1_000_000);
    const maxAmountB = new anchor.BN(1_000_000);

    const userTokenAAccount = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      userSigner
    );

    const userTokenBAccount = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      userSigner
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
        await program.methods
          .deposit(lpAmount, maxAmountA, maxAmountB)
          .accounts({
            userSigner,
            vaultAccount,
            vaultLpTokenMintPubkey,
            vaultInputTokenAAccount,
            vaultInputTokenBAccount,
            userLpTokenAccount,
            userTokenAAccount,
            userTokenBAccount,
            whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
            whTokenVaultA: poolData.tokenVaultA,
            whTokenVaultB: poolData.tokenVaultB,
            position: positionAccounts,
          })
          .transaction()
      );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit", txSig);
  });

  it("Deposit with tokens in vault", async () => {
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    const lpAmount = new anchor.BN(2_000_000);
    const maxAmountA = new anchor.BN(1_000_000);
    const maxAmountB = new anchor.BN(1_000_000);

    const userTokenAAccount = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      userSigner
    );

    const userTokenBAccount = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      userSigner
    );

    const userLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      userSigner
    );

    const transferIx = createTransferInstruction(
      userTokenAAccount,
      vaultInputTokenAAccount,
      userSigner,
      1_000,
      []
    );

    const tx = new anchor.web3.Transaction().add(transferIx).add(
      await program.methods
        .deposit(lpAmount, maxAmountA, maxAmountB)
        .accounts({
          userSigner,
          vaultAccount,
          vaultLpTokenMintPubkey,
          vaultInputTokenAAccount,
          vaultInputTokenBAccount,
          userLpTokenAccount,
          userTokenAAccount,
          userTokenBAccount,
          whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
          whTokenVaultA: poolData.tokenVaultA,
          whTokenVaultB: poolData.tokenVaultB,
          position: positionAccounts,
        })
        .transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit_with_tokens_in_vault", txSig);
  });

  it("Collect fees & rewards", async () => {
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    const liquidityAmount = new anchor.BN(1_000_000);
    const maxAmountA = new anchor.BN(1_000_000);
    const maxAmountB = new anchor.BN(1_000_000);

    const tokenOwnerAccountA = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      userSigner
    );

    const tokenOwnerAccountB = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      userSigner
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
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    const tokenOwnerAccountA = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      userSigner
    );

    const tokenOwnerAccountB = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      userSigner
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
          currentPosition: positionAccounts,
          newPosition: positionAccounts2,
        })
        .transaction()
    );

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("rebalance", txSig);
  });

  it("Withdraw", async () => {
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    const liquidityAmount = new anchor.BN(1_000);
    const minAmountA = new anchor.BN(0);
    const minAmountB = new anchor.BN(0);

    const tokenOwnerAccountA = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      userSigner
    );

    const tokenOwnerAccountB = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      userSigner
    );

    const tx = await program.methods
      .withdraw(liquidityAmount, minAmountA, minAmountB)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        tokenOwnerAccountA,
        tokenOwnerAccountB,
        tokenVaultA: poolData.tokenVaultA,
        tokenVaultB: poolData.tokenVaultB,
        position: positionAccounts2,
      })
      .transaction();

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("withdraw", txSig);
  });
});

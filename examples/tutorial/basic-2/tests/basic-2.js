const assert = require("assert");
const anchor = require('@project-serum/anchor');

describe("basic-2", () => {
  const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  // Author for the tests.
  const author = new anchor.web3.Account();

  // Program for the tests.
  const program = anchor.workspace.Basic2;

  it("Creates an author", async () => {
    await program.rpc.createAuthor(provider.wallet.publicKey, "Ghost", {
      accounts: {
        author: author.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [author],
      instructions: [
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: author.publicKey,
          space: 8 + 1000,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(
            8 + 1000
          ),
          programId: program.programId,
        }),
      ],
    });

    let authorAccount = await program.account.author(author.publicKey);

    assert.ok(authorAccount.authority.equals(provider.wallet.publicKey));
    assert.ok(authorAccount.name === "Ghost");
  });

  it("Updates an author", async () => {
    await program.rpc.updateAuthor("Updated author", {
      accounts: {
        author: author.publicKey,
        authority: provider.wallet.publicKey,
      },
    });

    authorAccount = await program.account.author(author.publicKey);

    assert.ok(authorAccount.authority.equals(provider.wallet.publicKey));
    assert.ok(authorAccount.name === "Updated author");
  });

  // Book params to use accross tests.
  const book = new anchor.web3.Account();
  const pages = [
    {
      content: "first page",
      footnote: "first footnote",
    },
    {
      content: "second page",
      footnote: "second footnote",
    },
  ];

  it("Creates a book", async () => {
    await program.rpc.createBook("Book title", pages, {
      accounts: {
        authority: provider.wallet.publicKey,
        author: author.publicKey,
        book: book.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [book],
      instructions: [
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: book.publicKey,
          space: 8 + 1000,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(
            8 + 1000
          ),
          programId: program.programId,
        }),
      ],
    });

    const bookAccount = await program.account.book(book.publicKey);

    assert.ok(bookAccount.author.equals(author.publicKey));
    assert.ok(bookAccount.title === "Book title");
    assert.deepEqual(bookAccount.pages, pages);
  });

  it("Updates a book", async () => {
    await program.rpc.updateBook("New book title", null, {
      accounts: {
        authority: provider.wallet.publicKey,
        author: author.publicKey,
        book: book.publicKey,
      },
    });

    const bookAccount = await program.account.book(book.publicKey);

    assert.ok(bookAccount.author.equals(author.publicKey));
    assert.ok(bookAccount.title === "New book title");
    assert.deepEqual(bookAccount.pages, pages);
  });
});

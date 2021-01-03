# Introduction

Anchor is a framework for Solana's [Sealevel](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192) runtime, exposing a safer and more convenient programming model to the Solana developer by providing a

- Rust Crate for writing Solana programs
- CLI for extracting an [IDL](https://en.wikipedia.org/wiki/Interface_description_language) from source
- TypeScript package for generating clients from IDL

If you're familiar with developing in Ethereum's [Solidity](https://docs.soliditylang.org/en/v0.7.4/) and [web3.js](https://github.com/ethereum/web3.js) or Parity's [Ink!](https://github.com/paritytech/ink), then the experience will be familiar. Although the DSL syntax and semantics are targeted at Solana, the high level flow of writing RPC request handlers, emitting an IDL, and generating clients from IDL is the same.

Here, we'll walkthrough a tutorial demonstrating how to use Anchor. To skip the tutorial and jump straight to a full example, go [here](https://github.com/project-serum/anchor/tree/master/examples/basic).

## Contributing

It would be great to have clients generated for languages other than TypeScript. If you're
interested in developing a client generator, feel free to reach out, or go ahead and just
do it :P.

## Note

Anchor is in active development, so all APIs are subject to change. If you have feedback, please reach out by [filing an issue](https://github.com/project-serum/anchor/issues/new). This documentation is a work in progress and is expected to change dramatically as features continue to be built out. If you have any problems, consult the [source](https://github.com/project-serum/anchor) or feel free to ask questions on the [Serum Discord](https://discord.com/channels/739225212658122886/752530209848295555).

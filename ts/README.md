# @project-serum/anchor

[![npm](https://img.shields.io/npm/v/@project-serum/anchor.svg?color=blue)](https://www.npmjs.com/package/@project-serum/anchor)
[![Docs](https://img.shields.io/badge/docs-typedoc-blue)](https://coral-xyz.github.io/anchor/ts/index.html)

TypeScript client for Anchor programs.

## Note

* `@project-serum/anchor` depends on node.js native modules. Therefore, webpack 5 will not work with current version. You will either need to rollback to webpack 4, or use a polyfill for each missing dependency.
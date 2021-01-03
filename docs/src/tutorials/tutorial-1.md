# Tutorial 1: Accounts, Arguments, and Types

It's recommended to read [Tutorial 0](./tutorial-0.md) first, as this tutorial will
build on top of it. The full example can be found [here](https://github.com/project-serum/anchor/tree/master/examples/basic-1).

## Defining a program

We define our program as follows

<<< @/../examples/tutorial/basic-1/program/src/lib.rs#program

Some new syntax elements are introduced here.

First notice, the `data` argument passed into the program. This argument any other valid
Rust types can be passed to the instruction to define inputs to the program. Additionally,
notice how we take a `mutable` reference to `my_account` and assign to it.

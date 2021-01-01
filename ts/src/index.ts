import { PublicKey } from '@solana/web3.js';

export class Program {
	/**
	 * Address of the program.
	 */
	public programId: PublicKey;

	/**
	 * The inner variables required to implement the Program object.
	 */
	public _inner: ProgramInner;

	public constructor(idl: Idl, programId: PublicKey, options?: ProgramOptions) {
		this.programId = programId;
		this._inner = {
			options: options === undefined ? {} : options,
		};
		console.log("building",idl);
	}
}

type Idl = any;

type ProgramInner = {
	options: ProgramOptions;
}

type ProgramOptions = {};

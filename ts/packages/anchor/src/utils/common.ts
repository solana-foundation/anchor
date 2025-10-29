import { Transaction, VersionedTransaction } from "@solana/web3.js";
import camelCase from "camelcase";


/**
 * Returns true if being run inside a web browser,
 * false if in a Node process or electron app.
 */
export const isBrowser =
  process.env.ANCHOR_BROWSER ||
  (typeof window !== "undefined" && !window.process?.hasOwnProperty("type"));

/**
 * Splits an array into chunks
 *
 * @param array Array of objects to chunk.
 * @param size The max size of a chunk.
 * @returns A two dimensional array where each T[] length is < the provided size.
 */
export function chunks<T>(array: T[], size: number): T[][] {
  return Array.apply(0, new Array(Math.ceil(array.length / size))).map(
    (_, index) => array.slice(index * size, (index + 1) * size)
  );
}

/**
 * Check if a transaction object is a VersionedTransaction or not
 *
 * @param tx
 * @returns bool
 */
export const isVersionedTransaction = (
  tx: Transaction | VersionedTransaction
): tx is VersionedTransaction => {
  return "version" in tx;
};

const NUMBER_LETTER_PATTERN = /(\d)([a-zA-Z])/g;

/**
 * Harmonized camelCase conversion that handles number+letter patterns consistently
 * with Rust's heck library behavior.
 * 
 * This fixes the discrepancy between Rust (heck) and JavaScript (camelcase) libraries
 * when converting identifiers with numbers followed by letters (e.g., a1b_receive → a1BReceive).
 *
 * @param input The string to convert to camelCase
 * @returns The harmonized camelCase string
 */
export function harmonizedCamelCase(input: string): string {
  // Apply normal camelCase first
  let result = camelCase(input, { locale: false });

  
  
  // Fix number+letter patterns to match Rust side (a1b → a1B)
  return result.replace(NUMBER_LETTER_PATTERN, (match, digit, letter) => {
    return digit + letter.toUpperCase();
  });
}
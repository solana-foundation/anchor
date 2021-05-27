export function decode(array: Uint8Array): string {
  const decoder =
    typeof TextDecoder === "undefined"
      ? new (require("util").TextDecoder)("utf-8") // Node.
      : new TextDecoder("utf-8"); // Browser.
  return decoder.decode(array);
}

export function encode(input: string): Uint8Array {
  const encoder =
    typeof TextEncoder === "undefined"
      ? new (require("util").TextEncoder)("utf-8") // Node.
      : new TextEncoder(); // Browser.
  return encoder.encode(input);
}

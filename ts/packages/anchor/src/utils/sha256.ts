import { sha256 } from "js-sha256";

export function hash(data: string): string {
  return sha256(data);
}

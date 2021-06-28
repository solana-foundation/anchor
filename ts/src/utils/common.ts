/**
 * Returns true if being run inside a web browser,
 * false if in a Node process or electron app.
 */
export const isBrowser =
  typeof window !== "undefined" && !window.process?.hasOwnProperty("type");

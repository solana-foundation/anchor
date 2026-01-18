// https://stackoverflow.com/a/13227808
export function getCaller(): string {
  const stack = getStack();
  const caller = stack[2]!.getFileName();
  if (!caller) throw new Error("could not determine caller");
  return caller;
}

export function getStack() {
  const error = new Error();

  const origPrepareStackTrace = Error.prepareStackTrace;
  Error.prepareStackTrace = function (_, stack) {
    return stack;
  };
  const stack = error.stack as unknown as NodeJS.CallSite[];
  Error.prepareStackTrace = origPrepareStackTrace;
  stack.shift();

  return stack;
}

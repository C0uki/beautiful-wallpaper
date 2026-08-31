// Turning whatever was thrown into a sentence.
//
// The backend rejects with the string its command returned, so most of the
// time there is nothing to do. But a transport failure throws a real `Error`,
// and `String(error)` on one of those prefixes it with "Error:" — which is
// noise in a panel that is already labelled as a problem.

export function describeError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return String(error);
}

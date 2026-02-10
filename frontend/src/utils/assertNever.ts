/**
 * Exhaustive match guard. Place in the `default` case of a switch on a
 * discriminated union to get a compile-time error when a variant is unhandled.
 *
 * At runtime, throws if somehow reached (e.g. new variant added at server
 * but client not yet updated).
 */
const assertNever = (value: never): never => {
  throw new Error(`Unhandled discriminated union member: ${JSON.stringify(value)}`)
}

export { assertNever }

/** Narrow partial test doubles to mocked runtime types under type-aware lint. */
export function castMock<T>(value: unknown, _typeHint?: T): T {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- partial mocks in unit tests
  return value as T
}

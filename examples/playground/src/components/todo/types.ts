export interface Todo {
  readonly id: string
  readonly text: string
  readonly completed: boolean
  readonly createdAt: string
}

export interface TodoActionState {
  readonly success: boolean
  readonly error?: string
  readonly todos?: readonly Todo[]
}

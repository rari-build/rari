import { useCallback, useEffect, useRef, useState, useTransition } from 'react'

export type ActionFunction<State, Payload = FormData> = (
  prevState: State,
  payload: Payload
) => Promise<State>

export type ActionState<State> = [
  state: State,
  dispatch: (payload: FormData) => void,
  isPending: boolean,
]

export function useActionState<State>(
  action: ActionFunction<State>,
  initialState: State,
  _permalink?: string,
): ActionState<State> {
  const [state, setState] = useState<State>(initialState)
  const [isPending, startTransition] = useTransition()
  const currentStateRef = useRef<State>(initialState)

  useEffect(() => {
    currentStateRef.current = state
  }, [state])

  const dispatch = useCallback(
    (payload: FormData) => {
      startTransition(async () => {
        try {
          const newState = await action(currentStateRef.current, payload)
          setState(newState)
        }
        catch (error) {
          console.error('Action failed:', error)
        }
      })
    },
    [action],
  )

  return [state, dispatch, isPending]
}

export function useActionStateWithFallback<State>(
  action: ActionFunction<State>,
  initialState: State,
  permalink?: string,
): ActionState<State> & { formAction?: string } {
  const [state, dispatch, isPending] = useActionState(action, initialState, permalink)

  return {
    0: state,
    1: dispatch,
    2: isPending,
    formAction: permalink,
    * [Symbol.iterator]() {
      yield state
      yield dispatch
      yield isPending
    },
  } as ActionState<State> & { formAction?: string }
}

export function useOptimisticAction<State, OptimisticState = State>(
  action: ActionFunction<State>,
  initialState: State,
  optimisticReducer: (state: State, payload: FormData) => OptimisticState,
): [
  state: OptimisticState,
  dispatch: (payload: FormData) => void,
  isPending: boolean,
] {
  const [actualState, setActualState] = useState<State>(initialState)
  const [optimisticState, setOptimisticState] = useState<OptimisticState>(
    initialState as unknown as OptimisticState,
  )
  const [isPending, startTransition] = useTransition()

  const dispatch = useCallback(
    (payload: FormData) => {
      const newOptimisticState = optimisticReducer(actualState, payload)
      setOptimisticState(newOptimisticState)

      startTransition(async () => {
        try {
          const newActualState = await action(actualState, payload)
          setActualState(newActualState)
          setOptimisticState(newActualState as unknown as OptimisticState)
        }
        catch (error) {
          setOptimisticState(actualState as unknown as OptimisticState)
          console.error('Action failed:', error)
        }
      })
    },
    [action, actualState, optimisticReducer],
  )

  return [optimisticState, dispatch, isPending]
}

export function useValidatedAction<State>(
  action: ActionFunction<State>,
  initialState: State,
  validator?: (formData: FormData) => string[] | null,
): ActionState<State> & { errors: string[] } {
  const [errors, setErrors] = useState<string[]>([])

  const validatedAction = useCallback(
    async (prevState: State, formData: FormData): Promise<State> => {
      setErrors([])

      if (validator) {
        const validationErrors = validator(formData)
        if (validationErrors && validationErrors.length > 0) {
          setErrors(validationErrors)
          return prevState
        }
      }

      try {
        return await action(prevState, formData)
      }
      catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error'
        setErrors([errorMessage])
        return prevState
      }
    },
    [action, validator],
  )

  const [state, dispatch, isPending] = useActionState(validatedAction, initialState)

  return {
    0: state,
    1: dispatch,
    2: isPending,
    errors,
    * [Symbol.iterator]() {
      yield state
      yield dispatch
      yield isPending
    },
  } as ActionState<State> & { errors: string[] }
}

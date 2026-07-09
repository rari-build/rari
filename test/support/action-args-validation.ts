import '../../crates/rari/src/rendering/base/js/action_args_validation.core'

export interface ActionValidationConfig {
  maxDepth: number
  maxStringLength: number
  maxArrayLength: number
  maxObjectKeys: number
  maxTotalElements: number
}

interface ActionArgsValidationApi {
  productionValidationConfig: () => ActionValidationConfig
  developmentValidationConfig: () => ActionValidationConfig
  validateActionArgsWithConfig: (args: unknown[], config: ActionValidationConfig) => unknown[]
  validateFormDataWithConfig: (formData: FormData, config: ActionValidationConfig) => void
  isDangerousActionProperty: (key: string) => boolean
}

function getActionArgsValidationApi(): ActionArgsValidationApi {
  const api = (globalThis as typeof globalThis & {
    __RARI_ACTION_ARGS_VALIDATION__?: ActionArgsValidationApi
  }).__RARI_ACTION_ARGS_VALIDATION__

  if (!api)
    throw new Error('action_args_validation.core failed to initialize')

  return api
}

const validationApi = getActionArgsValidationApi()

export const productionValidationConfig = validationApi.productionValidationConfig
export const developmentValidationConfig = validationApi.developmentValidationConfig

export function validateActionArgs(args: unknown[], config: ActionValidationConfig): unknown[] {
  return getActionArgsValidationApi().validateActionArgsWithConfig(args, config)
}

export function validateFormData(formData: FormData, config: ActionValidationConfig): void {
  getActionArgsValidationApi().validateFormDataWithConfig(formData, config)
}

/// <reference path="../../crates/rari/src/rendering/types.d.ts" />
import '../../crates/rari/src/rendering/base/js/action_args_validation.core'

interface ActionArgsValidationApi {
  productionValidationConfig: () => ActionValidationConfig
  developmentValidationConfig: () => ActionValidationConfig
  validateActionArgsWithConfig: (
    args: readonly unknown[],
    config: ActionValidationConfig,
  ) => unknown[]
  validateFormDataWithConfig: (formData: FormData, config: ActionValidationConfig) => void
  isDangerousActionProperty: (key: string) => boolean
}

function getActionArgsValidationApi(): ActionArgsValidationApi {
  const api = (
    globalThis as typeof globalThis & {
      __RARI_ACTION_ARGS_VALIDATION__?: ActionArgsValidationApi
    }
  ).__RARI_ACTION_ARGS_VALIDATION__

  if (!api) throw new Error('action_args_validation.core failed to initialize')

  return api
}

const validationApi = getActionArgsValidationApi()

export const productionValidationConfig = validationApi.productionValidationConfig
export const developmentValidationConfig = validationApi.developmentValidationConfig

export function validateActionArgs(
  args: readonly unknown[],
  config: ActionValidationConfig,
): unknown[] {
  return getActionArgsValidationApi().validateActionArgsWithConfig(args, config)
}

export function validateFormData(formData: FormData, config: ActionValidationConfig): void {
  getActionArgsValidationApi().validateFormDataWithConfig(formData, config)
}

/// <reference path="../../types.d.ts" />
/* eslint-disable unused-imports/no-unused-vars -- concatenated into action_handler.ts */

function getActionArgsValidationApi() {
  const api = (globalThis as typeof globalThis & {
    __RARI_ACTION_ARGS_VALIDATION__?: {
      productionValidationConfig: () => unknown
      developmentValidationConfig: () => unknown
      validateActionArgsWithConfig: (args: unknown[], config: unknown) => unknown[]
      validateFormDataWithConfig: (formData: FormData, config: unknown) => void
    }
  }).__RARI_ACTION_ARGS_VALIDATION__

  if (!api)
    throw new TypeError('Action args validation API not initialized')

  return api
}

function actionValidationConfig() {
  const api = getActionArgsValidationApi()
  return g['~rari']?.isDevelopment
    ? api.developmentValidationConfig()
    : api.productionValidationConfig()
}

function validateActionArgs(args: unknown[]): unknown[] {
  return getActionArgsValidationApi().validateActionArgsWithConfig(args, actionValidationConfig())
}

function validateFormData(formData: FormData): void {
  getActionArgsValidationApi().validateFormDataWithConfig(formData, actionValidationConfig())
}

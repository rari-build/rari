/// <reference path="../../types.d.ts" />

function getActionArgsValidationApi(): ActionArgsValidationApi {
  const api = g.__RARI_ACTION_ARGS_VALIDATION__

  if (!api) throw new TypeError('Action args validation API not initialized')

  return api
}

function actionValidationConfig() {
  const api = getActionArgsValidationApi()
  return g['~rari']?.isDevelopment
    ? api.developmentValidationConfig()
    : api.productionValidationConfig()
}

function validateActionArgs(args: readonly unknown[]): unknown[] {
  return getActionArgsValidationApi().validateActionArgsWithConfig(args, actionValidationConfig())
}

function validateFormData(formData: FormData): void {
  getActionArgsValidationApi().validateFormDataWithConfig(formData, actionValidationConfig())
}

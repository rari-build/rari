/// <reference path="../../types.d.ts" />
/* eslint-disable unused-imports/no-unused-vars -- concatenated into action_handler.ts */

function actionValidationConfig() {
  return g['~rari']?.isDevelopment
    ? developmentValidationConfig()
    : productionValidationConfig()
}

function validateActionArgs(args: unknown[]): unknown[] {
  return validateActionArgsWithConfig(args, actionValidationConfig())
}

function validateFormData(formData: FormData): void {
  validateFormDataWithConfig(formData, actionValidationConfig())
}

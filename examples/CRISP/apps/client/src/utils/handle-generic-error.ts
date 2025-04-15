export const handleGenericError = (functionName: string, error: Error) => {
  console.error(`[${functionName}] - ${error.message}`)
  // throw new Error(`[${functionName}] -  ${error.message}`)
}

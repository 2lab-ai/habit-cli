class CliError extends Error {
  /**
   * @param {string} message
   * @param {number} exitCode
   */
  constructor(message, exitCode) {
    super(message);
    this.name = 'CliError';
    this.exitCode = exitCode;
  }
}

function usageError(message) {
  return new CliError(message, 2);
}

function notFoundError(message) {
  return new CliError(message, 3);
}

function ambiguousError(message) {
  return new CliError(message, 4);
}

function ioError(message) {
  return new CliError(message, 5);
}

module.exports = {
  CliError,
  usageError,
  notFoundError,
  ambiguousError,
  ioError
};

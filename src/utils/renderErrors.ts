import { logger } from './logger';

export interface FriendlyRenderError {
  message: string;
  details: string;
}

const renderErrorLogger = logger.createScoped('renderErrors');

const TIKZ_KEYWORDS = [
  'tikz',
  'pgf',
  'undefined control sequence',
  'lualatex',
  'pdflatex',
];

function normalizeErrorPayload(err: unknown): string {
  if (err instanceof Error) {
    return err.stack ?? err.message;
  }

  if (typeof err === 'string') {
    return err;
  }

  try {
    return JSON.stringify(err);
  } catch (jsonErr) {
    renderErrorLogger.debug('Failed to stringify render error payload', jsonErr);
    return String(err);
  }
}

export function deriveRenderError(err: unknown, fallbackMessage: string): FriendlyRenderError {
  const raw = normalizeErrorPayload(err);
  const normalized = raw.toLowerCase();

  if (TIKZ_KEYWORDS.some((keyword) => normalized.includes(keyword))) {
    return {
      message: 'TikZ render failed – check your diagram syntax or preamble',
      details: raw,
    };
  }

  return {
    message: fallbackMessage,
    details: raw,
  };
}

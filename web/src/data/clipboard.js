/**
 * Safely copy text to clipboard across secure and insecure browser contexts.
 *
 * 1. Checks if modern navigator.clipboard is available in a secure context.
 * 2. Falls back to document.execCommand('copy') using a temporary off-screen textarea.
 * 3. Falls back to window.prompt if promptFallback is true.
 *
 * @param {string} text - Text to copy
 * @param {{
 *   promptFallback?: boolean,
 *   targetWindow?: any,
 *   targetDocument?: any,
 *   targetNavigator?: any,
 * }} [options]
 * @returns {Promise<boolean>}
 */
export async function copyTextToClipboard(
  text,
  {
    promptFallback = true,
    targetWindow = typeof window !== 'undefined' ? window : undefined,
    targetDocument = typeof document !== 'undefined' ? document : undefined,
    targetNavigator = typeof navigator !== 'undefined' ? navigator : undefined,
  } = {},
) {
  if (!text || typeof text !== 'string') return false;

  // 1. Try modern Clipboard API (available in secure contexts: HTTPS, localhost)
  if (
    targetWindow?.isSecureContext &&
    targetNavigator?.clipboard &&
    typeof targetNavigator.clipboard.writeText === 'function'
  ) {
    try {
      await targetNavigator.clipboard.writeText(text);
      return true;
    } catch {
      // In case writeText fails or permission is denied, fall through to execCommand
    }
  }

  // 2. Fallback to document.execCommand('copy') with temporary textarea
  if (targetDocument?.body && typeof targetDocument.createElement === 'function') {
    try {
      const textarea = targetDocument.createElement('textarea');
      textarea.value = text;
      if (typeof textarea.setAttribute === 'function') {
        textarea.setAttribute('readonly', '');
      }
      if (textarea.style) {
        textarea.style.position = 'fixed';
        textarea.style.top = '0';
        textarea.style.left = '-9999px';
        textarea.style.opacity = '0';
        textarea.style.pointerEvents = 'none';
      }

      targetDocument.body.appendChild(textarea);
      if (typeof textarea.focus === 'function') textarea.focus();
      if (typeof textarea.select === 'function') textarea.select();
      if (typeof textarea.setSelectionRange === 'function') {
        textarea.setSelectionRange(0, text.length);
      }

      const successful =
        typeof targetDocument.execCommand === 'function'
          ? targetDocument.execCommand('copy')
          : false;

      targetDocument.body.removeChild(textarea);

      if (successful) return true;
    } catch {
      // execCommand failed, fall through to prompt
    }
  }

  // 3. Fallback to window.prompt for manual copy in restricted environments
  if (promptFallback && typeof targetWindow?.prompt === 'function') {
    try {
      targetWindow.prompt('请手动复制链接：', text);
      return true;
    } catch {
      // ignore
    }
  }

  return false;
}

export function cloneJson(value) {
  if (value === undefined) return undefined;
  return JSON.parse(JSON.stringify(value));
}

export function readOther(draftOrText) {
  const raw =
    typeof draftOrText === 'string'
      ? draftOrText
      : typeof draftOrText === 'object' && draftOrText !== null
        ? draftOrText.otherText
        : '';
  const text = String(raw ?? '').trim();
  if (!text) return {};
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error('其他字段 JSON 不合法');
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('其他字段必须是 JSON 对象');
  }
  return parsed;
}

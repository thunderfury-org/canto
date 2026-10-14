export const AUTH_CHECKING = 'checking';
export const AUTH_REQUIRED = 'required';
export const AUTH_ENTERED = 'entered';

export const PASSWORD_INCORRECT = '密码不正确';
export const CANNOT_REACH_CANTO = '连不上 canto';
export const PASSWORD_NOT_SET = '网关还没有设置密码';
export const CANNOT_EXIT = '退不出去';

const NOT_CONFIGURED = 'Admin token is not configured on server';

export function authMessage(status, body) {
  if (status === 0) return CANNOT_REACH_CANTO;
  if (body?.message === NOT_CONFIGURED) return PASSWORD_NOT_SET;
  return PASSWORD_INCORRECT;
}

export function enteredStudio(status) {
  return status === 200;
}

export function emptyPassword(value) {
  return String(value ?? '').trim() === '';
}

export function silentAuthMessage(status, body) {
  if (status === 0) return CANNOT_REACH_CANTO;
  if (body?.message === NOT_CONFIGURED) return PASSWORD_NOT_SET;
  return '';
}

export function verifyBody(password) {
  return JSON.stringify(password === undefined ? {} : { token: password });
}

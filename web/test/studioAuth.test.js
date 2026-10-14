import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  CANNOT_REACH_CANTO,
  PASSWORD_INCORRECT,
  PASSWORD_NOT_SET,
  authMessage,
  emptyPassword,
  enteredStudio,
  silentAuthMessage,
  verifyBody,
} from '../src/data/studioAuth.js';

test('a 200 verify response has entered the studio', () => {
  assert.equal(enteredStudio(200), true);
  assert.equal(enteredStudio(401), false);
  assert.equal(enteredStudio(0), false);
});

test('an unreachable verify names the connection, not the password', () => {
  assert.equal(authMessage(0, null), CANNOT_REACH_CANTO);
});

test('a gateway without a password says so', () => {
  assert.equal(
    authMessage(401, { message: 'Admin token is not configured on server' }),
    PASSWORD_NOT_SET,
  );
});

test('any other rejected password is incorrect', () => {
  assert.equal(authMessage(401, { message: 'Invalid admin token' }), PASSWORD_INCORRECT);
  assert.equal(authMessage(401, {}), PASSWORD_INCORRECT);
  assert.equal(authMessage(500, { message: 'boom' }), PASSWORD_INCORRECT);
});

test('a blank password cannot be submitted', () => {
  assert.equal(emptyPassword(''), true);
  assert.equal(emptyPassword('   '), true);
  assert.equal(emptyPassword(null), true);
  assert.equal(emptyPassword('secret'), false);
});

test('a silent verify hides a missing or rejected password', () => {
  assert.equal(silentAuthMessage(401, { message: 'Invalid admin token' }), '');
  assert.equal(silentAuthMessage(401, {}), '');
  assert.equal(silentAuthMessage(400, { message: 'Failed to parse' }), '');
});

test('a silent verify still names an unreachable gateway and a missing password', () => {
  assert.equal(silentAuthMessage(0, null), CANNOT_REACH_CANTO);
  assert.equal(
    silentAuthMessage(401, { message: 'Admin token is not configured on server' }),
    PASSWORD_NOT_SET,
  );
});

test('opening the studio verifies an empty json body', () => {
  assert.equal(verifyBody(undefined), '{}');
  assert.equal(verifyBody('secret'), '{"token":"secret"}');
});

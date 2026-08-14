import { test } from 'node:test';
import assert from 'node:assert';
import { ApiClient } from '../src/services/ApiClient';

test('ApiClient - Base URL and bearer token handling', () => {
  const client = new ApiClient('http://127.0.0.1:7890', 'test_secret_token_123');

  assert.strictEqual(client.getBaseUrl(), 'http://127.0.0.1:7890');
  assert.strictEqual(client.getToken(), 'test_secret_token_123');

  client.setToken('updated_token_456');
  assert.strictEqual(client.getToken(), 'updated_token_456');
});

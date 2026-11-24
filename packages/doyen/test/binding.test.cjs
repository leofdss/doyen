const assert = require('node:assert/strict');
const test = require('node:test');

const { raiz, saudacao } = require('../index.js');

test('raiz returns the original root response', () => {
  assert.equal(raiz(), 'Olá, mundo!');
});

test('saudacao returns the original personalized response', () => {
  assert.equal(saudacao('Maria'), 'Olá, Maria!');
  assert.equal(saudacao('João'), 'Olá, João!');
});

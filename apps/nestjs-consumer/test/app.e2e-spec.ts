import { INestApplication } from '@nestjs/common';
import { Test, TestingModule } from '@nestjs/testing';
import { realpathSync } from 'node:fs';
import { join } from 'node:path';
import request from 'supertest';
import { App } from 'supertest/types';
import { AppModule } from '../src/app.module';

describe('doyen package in NestJS (e2e)', () => {
  let app: INestApplication<App>;

  beforeEach(async () => {
    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();
  });

  afterEach(async () => {
    await app.close();
  });

  it('loads doyen from this app node_modules, not from the workspace', () => {
    const resolvedPackage = require.resolve('doyen/package.json');
    const installedPackage = join(
      __dirname,
      '..',
      'node_modules',
      'doyen',
      'package.json',
    );

    expect(realpathSync(resolvedPackage)).toBe(realpathSync(installedPackage));
  });

  it('/ (GET) executes the native root function', () => {
    return request(app.getHttpServer())
      .get('/')
      .expect(200)
      .expect('Olá, mundo!');
  });

  it('/saudacao/:nome (GET) executes the native greeting function', () => {
    return request(app.getHttpServer())
      .get('/saudacao/Maria')
      .expect(200)
      .expect('Olá, Maria!');
  });
});

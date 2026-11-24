import { Test, TestingModule } from '@nestjs/testing';
import { AppController } from './app.controller';
import { AppService } from './app.service';

describe('AppController', () => {
  let appController: AppController;

  beforeEach(async () => {
    const app: TestingModule = await Test.createTestingModule({
      controllers: [AppController],
      providers: [AppService],
    }).compile();

    appController = app.get<AppController>(AppController);
  });

  it('returns the response from the packed native addon', () => {
    expect(appController.getRoot()).toBe('Olá, mundo!');
  });

  it('passes route parameters to the packed native addon', () => {
    expect(appController.getGreeting('Maria')).toBe('Olá, Maria!');
  });
});

import { Controller, Get, Param } from '@nestjs/common';
import { AppService } from './app.service';

@Controller()
export class AppController {
  constructor(private readonly appService: AppService) {}

  @Get()
  getRoot(): string {
    return this.appService.getRoot();
  }

  @Get('saudacao/:nome')
  getGreeting(@Param('nome') name: string): string {
    return this.appService.getGreeting(name);
  }
}

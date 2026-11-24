import { Injectable } from '@nestjs/common';
import { raiz, saudacao } from 'doyen';

@Injectable()
export class AppService {
  getRoot(): string {
    return raiz();
  }

  getGreeting(name: string): string {
    return saudacao(name);
  }
}

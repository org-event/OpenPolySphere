import { exposeWindowApi } from './ui/window-api.js';
import { startApp } from './boot.js';

exposeWindowApi();
startApp();

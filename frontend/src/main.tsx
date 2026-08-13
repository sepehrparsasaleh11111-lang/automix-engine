import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/index.css';

const bootstrap = async () => {
  if (import.meta.env.VITE_E2E === 'true') {
    const { installMocks } = await import('./e2e/mocks');
    installMocks();
  }
  ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
};

bootstrap();
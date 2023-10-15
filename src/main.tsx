import React from 'react'
import ReactDOM from 'react-dom/client'
import './index.css'
import 'bootstrap/dist/css/bootstrap.min.css';
import { RouterProvider, createBrowserRouter } from 'react-router-dom'
import Root from './routes/root';
import PadEdit from './routes/pad-edit';
import App from './routes/app';
import CipherpadProvider from './providers/CipherpadProvider';
import PadBlobView from './routes/pad-blob-view';

const router = createBrowserRouter([
  {
    path: '/',
    element: <Root />,
  },
  {
    path: '/app',
    element: <App />
  },
  {
    path: '/pad-edit',
    element: <PadEdit />
  },
  {
    path: '/pad-blob-view',
    element: <PadBlobView />
  }
]);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <CipherpadProvider>
      <RouterProvider router={router} />
    </CipherpadProvider>
  </React.StrictMode>,
)

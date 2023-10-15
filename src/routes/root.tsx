import { open, save } from '@tauri-apps/api/dialog';
import { useNavigate } from 'react-router-dom';
import { useCipherpad } from '../providers/CipherpadProvider';
import Form from 'react-bootstrap/Form';
import Container from 'react-bootstrap/Container';
import { useState } from 'react';

export default function Root() {
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [lastError, setLastError] = useState<string | undefined>(undefined);
  const navigate = useNavigate();
  const { openOrCreateCipherpadWithPassword } = useCipherpad();

  const onOpenClicked = async () => {
    try {
      const cipherpadToTryOpen = await open();
      setLoading(true);
      if (cipherpadToTryOpen !== null && !Array.isArray(cipherpadToTryOpen)) {
        await openOrCreateCipherpadWithPassword(cipherpadToTryOpen, password);
        navigate('/app');
      }
    }
    catch (err) {
      let errorMessage;
      if (err instanceof Error) {
        errorMessage = err.message;
      } else {
        errorMessage = String(err);
      }
      setLastError(errorMessage);
    }
    setLoading(false);
  }

  const onCreateClicked = async () => {
    try {
      const cipherpadToTryCreate = await save({
        defaultPath: 'cipherpad.db',
        filters: [{
          name: 'Cipherpad',
          extensions: ['db']
        }]
      });
      setLoading(true);
      if (cipherpadToTryCreate !== null) {
        await openOrCreateCipherpadWithPassword(cipherpadToTryCreate, password);
        navigate('/app');
      }
    }
    catch (e) {
      setLastError(e instanceof Error ? e.message : String(e));
    }
    setLoading(false);
  }

  if (loading) return (<p>Loading...</p>);
  return (
    <Container>
      <Form>
        <Form.Group className='mb-3'>
          <Form.Label>Password</Form.Label>
          <Form.Control
            type='password'
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </Form.Group>
      </Form>
      <p>{lastError}</p>
      <button type='button' onClick={onOpenClicked}>Open</button>
      <button type='button' onClick={onCreateClicked}>Create</button>
    </Container>
  )
}
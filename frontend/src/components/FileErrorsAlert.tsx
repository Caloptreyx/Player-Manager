import { faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Alert from '@/elements/feedback/Alert.tsx';
import Code from '@/elements/typography/Code.tsx';
import type { FileError } from '../lib/model.ts';
import { useText } from './playerManager.ts';

// list files that exist but could not be parsed; missing files are not errors
export default function FileErrorsAlert({ errors }: { errors: FileError[] }) {
  const text = useText();
  if (errors.length === 0) return null;

  return (
    <Alert
      color='red'
      icon={<FontAwesomeIcon icon={faTriangleExclamation} />}
      title={text('errors.filesTitle', {})}
      className='text-sm!'
    >
      <div className='flex flex-col gap-1 text-sm'>
        <span>{text('errors.filesContent', {})}</span>
        <ul className='flex flex-col gap-0.5'>
          {errors.map((error) => (
            <li key={error.file}>
              <Code>{error.file}</Code> {error.message}
            </li>
          ))}
        </ul>
      </div>
    </Alert>
  );
}

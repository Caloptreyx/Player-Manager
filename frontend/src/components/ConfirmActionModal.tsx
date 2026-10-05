import { type FormEvent, useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { validateReason } from '../lib/validation.ts';
import { type ConfirmRequest, useText } from './playerManager.ts';

// confirmation of a destructive action (kick, ban, remove), optionally asking for a reason; stays open when
// the action fails so the user can retry
export default function ConfirmActionModal({
  request,
  run,
  onClose,
}: {
  request: ConfirmRequest | null;
  run: (request: ConfirmRequest, reason: string) => Promise<boolean>;
  onClose: () => void;
}) {
  const { t } = useTranslations();
  const text = useText();
  const [reason, setReason] = useState('');
  const [loading, setLoading] = useState(false);

  const reasonError = request?.withReason ? validateReason(reason.trim()) : null;

  const close = () => {
    setReason('');
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!request || reasonError) return;

    setLoading(true);
    const ok = await run(request, reason.trim());
    setLoading(false);
    if (ok) close();
  };

  return (
    <Modal title={request?.title} opened={request !== null} onClose={() => !loading && close()}>
      <form onSubmit={submit} className='flex flex-col gap-3'>
        <Text>{request?.content}</Text>
        {request?.withReason && (
          <TextInput
            label={text('form.reason', {})}
            placeholder={text('form.reasonPlaceholder', {})}
            value={reason}
            error={reasonError && text(`errors.${reasonError}`, {})}
            data-autofocus
            onChange={(e) => setReason(e.currentTarget.value)}
          />
        )}

        <ModalFooter>
          <Button type='submit' color='red' loading={loading} disabled={reasonError !== null}>
            {request?.confirm}
          </Button>
          <Button variant='default' disabled={loading} onClick={close}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </form>
    </Modal>
  );
}

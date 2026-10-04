import { type FormEvent, useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { addIpBan } from '../api.ts';
import { type FieldError, validateIp, validateReason } from '../lib/validation.ts';
import { useExtTranslations } from '../translations.ts';
import { usePlayerManager } from './playerManager.ts';

export default function AddIpBanModal({ opened, onClose }: { opened: boolean; onClose: () => void }) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const { serverUuid, overview, run } = usePlayerManager();

  const [ip, setIp] = useState('');
  const [reason, setReason] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [loading, setLoading] = useState(false);

  const trimmedIp = ip.trim();
  const ipError: FieldError | null =
    validateIp(trimmedIp) ??
    (overview.ip_bans.some((ban) => ban.ip.toLowerCase() === trimmedIp.toLowerCase()) ? 'duplicate' : null);
  const reasonError = validateReason(reason.trim());

  const close = () => {
    setIp('');
    setReason('');
    setSubmitted(false);
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSubmitted(true);
    if (ipError || reasonError) return;

    setLoading(true);
    const ok = await run(
      () => addIpBan(serverUuid, { ip: trimmedIp, reason: reason.trim() || undefined }),
      tExt('toast.ipBanned', { ip: trimmedIp }),
    );
    setLoading(false);
    if (ok) close();
  };

  return (
    <Modal title={tExt('actions.banIp', {})} opened={opened} onClose={() => !loading && close()}>
      <form onSubmit={submit} className='flex flex-col gap-3'>
        <TextInput
          label={tExt('form.ip', {})}
          value={ip}
          error={submitted && ipError ? tExt(`errors.${ipError}`, {}) : null}
          data-autofocus
          onChange={(e) => setIp(e.currentTarget.value)}
        />
        <TextInput
          label={tExt('form.reason', {})}
          placeholder={tExt('form.reasonPlaceholder', {})}
          value={reason}
          error={reasonError && tExt(`errors.${reasonError}`, {})}
          onChange={(e) => setReason(e.currentTarget.value)}
        />

        <ModalFooter>
          <Button type='submit' color='red' loading={loading}>
            {tExt('actions.ban', {})}
          </Button>
          <Button variant='default' disabled={loading} onClick={close}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </form>
    </Modal>
  );
}

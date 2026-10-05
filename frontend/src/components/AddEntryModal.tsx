import { type FormEvent, useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Autocomplete from '@/elements/input/Autocomplete.tsx';
import Checkbox from '@/elements/input/Checkbox.tsx';
import Select from '@/elements/input/Select.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { addToList } from '../api.ts';
import { addBody, isDuplicate } from '../lib/lists.ts';
import type { ListSpec } from '../lib/model.ts';
import { knownId, suggestions } from '../lib/players.ts';
import { type FieldError, validateId, validateIp, validateName, validateReason } from '../lib/validation.ts';
import { useExtTranslations } from '../translations.ts';
import { LIST_STYLES } from './listStyles.ts';
import { usePlayerManager, useText } from './playerManager.ts';

// adds an entry to a list; only offers the fields the list spec accepts in the current mode
export default function AddEntryModal({
  spec,
  opened,
  onClose,
}: {
  spec: ListSpec;
  opened: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const text = useText();
  const { serverUuid, game, ui, overview, run } = usePlayerManager();

  const [name, setName] = useState('');
  const [id, setId] = useState('');
  const [ip, setIp] = useState('');
  const [reason, setReason] = useState('');
  // null: the level the spec defaults to, which may change with the server state while the form is open
  const [level, setLevel] = useState<string | null>(null);
  const [limitFlag, setLimitFlag] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [loading, setLoading] = useState(false);

  const { kind } = spec;
  const entries = overview.lists[kind] ?? [];
  const byIp = spec.target === 'ip';
  const withId = spec.id === 'optional';

  const trimmedName = name.trim();
  const trimmedId = id.trim();
  const trimmedIp = ip.trim();
  const trimmedReason = reason.trim();
  const knownPlayerId = byIp || trimmedId ? null : knownId(overview.known, trimmedName);
  const resolvedId = trimmedId || knownPlayerId;
  const duplicate = isDuplicate(spec, entries, { name: trimmedName, id: resolvedId, ip: trimmedIp });

  const nameError: FieldError | null = byIp
    ? null
    : (validateName(game, trimmedName) ?? (duplicate ? 'duplicate' : null));
  const idError = withId ? validateId(game, trimmedId) : null;
  const ipError: FieldError | null = byIp ? (validateIp(trimmedIp) ?? (duplicate ? 'duplicate' : null)) : null;
  const reasonError = spec.reason ? validateReason(trimmedReason) : null;
  const invalid = nameError !== null || idError !== null || ipError !== null || reasonError !== null;

  const errorText = (error: FieldError | null) => (submitted && error ? text(`errors.${error}`, {}) : null);
  const note = ui.addFormNote(tExt, spec);

  const close = () => {
    setName('');
    setId('');
    setIp('');
    setReason('');
    setLevel(null);
    setLimitFlag(false);
    setSubmitted(false);
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSubmitted(true);
    if (invalid) return;

    const body = addBody(spec, {
      name: trimmedName,
      id: resolvedId,
      ip: trimmedIp,
      level: level ?? undefined,
      bypasses_player_limit: limitFlag,
      reason: trimmedReason,
    });
    setLoading(true);
    const ok = await run(
      () => addToList(serverUuid, kind, body),
      text(`lists.${kind}.added`, { name: byIp ? trimmedIp : trimmedName }),
    );
    setLoading(false);
    if (ok) close();
  };

  return (
    <Modal title={text(`lists.${kind}.addTitle`, {})} opened={opened} onClose={() => !loading && close()}>
      <form onSubmit={submit} className='flex flex-col gap-3'>
        {byIp ? (
          <TextInput
            label={text('form.ip', {})}
            value={ip}
            error={errorText(ipError)}
            data-autofocus
            onChange={(e) => setIp(e.currentTarget.value)}
          />
        ) : (
          <Autocomplete
            label={text('form.name', {})}
            value={name}
            data={suggestions(overview.known, entries)}
            limit={20}
            error={errorText(nameError)}
            data-autofocus
            onChange={setName}
          />
        )}

        {withId && (
          <TextInput
            label={text('form.id', {})}
            description={knownPlayerId ? text('form.knownId', { id: knownPlayerId }) : text('form.idOptional', {})}
            placeholder={knownPlayerId ?? undefined}
            value={id}
            error={errorText(idError)}
            onChange={(e) => setId(e.currentTarget.value)}
          />
        )}

        {spec.levels && (
          <Select
            label={text('form.level', {})}
            value={level ?? spec.levels.default}
            data={spec.levels.options.map((option) => ({
              value: option,
              label: ui.levelLabel(tExt, option, 'option'),
            }))}
            onChange={(value) => setLevel(value)}
          />
        )}

        {spec.bypasses_player_limit && (
          <Checkbox
            label={text('form.bypassesLimit', {})}
            checked={limitFlag}
            onChange={(e) => setLimitFlag(e.currentTarget.checked)}
          />
        )}

        {note && (
          <Alert color='blue' className='text-sm!'>
            {note}
          </Alert>
        )}

        {spec.reason && (
          <TextInput
            label={text('form.reason', {})}
            placeholder={text('form.reasonPlaceholder', {})}
            value={reason}
            error={reasonError && text(`errors.${reasonError}`, {})}
            onChange={(e) => setReason(e.currentTarget.value)}
          />
        )}

        <ModalFooter>
          <Button type='submit' color={LIST_STYLES[kind].danger ? 'red' : undefined} loading={loading}>
            {text(`lists.${kind}.submit`, {})}
          </Button>
          <Button variant='default' disabled={loading} onClick={close}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </form>
    </Modal>
  );
}

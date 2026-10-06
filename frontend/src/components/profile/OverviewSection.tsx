import {
  faBed,
  faDrumstickBite,
  faFlask,
  faGamepad,
  faHeart,
  faLocationDot,
  faStar,
  type IconDefinition,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Progress } from '@mantine/core';
import { type ReactNode, useState } from 'react';
import Card from '@/elements/data-display/Card.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { GAMEMODES, type Gamemode, type Position } from '../../lib/model.ts';
import { formatTicksClock, iconFills, prettifyId, romanNumeral } from '../../lib/profiles.ts';
import { XP_LEVEL_MAX } from '../../lib/validation.ts';
import { useExtTranslations } from '../../translations.ts';
import { usePlayerManager, useText } from '../playerManager.ts';
import { GatedButton } from '../RowAction.tsx';
import { useProfileView } from './profileView.ts';

function Section({ title, icon, children }: { title: string; icon: IconDefinition; children: ReactNode }) {
  return (
    <Card padding='md' className='flex flex-col gap-3'>
      <span className='inline-flex items-center gap-2 text-xs font-semibold tracking-wide text-(--mantine-color-dimmed) uppercase'>
        <FontAwesomeIcon icon={icon} />
        {title}
      </span>
      {children}
    </Card>
  );
}

/** A HUD bar of icons (hearts, shanks) that fill in halves. */
function IconBar({
  fills,
  icon,
  color,
  label,
}: {
  fills: number[];
  icon: IconDefinition;
  color: string;
  label: string;
}) {
  return (
    <div role='img' aria-label={label} className='flex flex-wrap gap-0.5'>
      {fills.map((fill, index) => (
        <span key={index} className='relative inline-block text-base leading-none'>
          <FontAwesomeIcon icon={icon} className='text-(--mantine-color-default-border)' />
          {fill > 0 && (
            <span className='absolute inset-0 overflow-hidden' style={{ width: `${fill * 100}%`, color }}>
              <FontAwesomeIcon icon={icon} />
            </span>
          )}
        </span>
      ))}
    </div>
  );
}

function Unknown() {
  const text = useText();
  return <span className='text-sm text-(--mantine-color-dimmed)'>{text('profile.vitals.unknown', {})}</span>;
}

function Vitals() {
  const text = useText();
  const { profile } = useProfileView();
  const max = profile.max_health ?? 20;
  const round = (value: number) => Math.round(value * 10) / 10;

  return (
    <Section title={text('profile.vitals.health', {})} icon={faHeart}>
      {profile.health === null ? (
        <Unknown />
      ) : (
        <div className='flex flex-wrap items-center justify-between gap-x-3 gap-y-1'>
          <IconBar
            fills={iconFills(profile.health, max)}
            icon={faHeart}
            color='#e5332a'
            label={`${round(profile.health)} / ${round(max)}`}
          />
          <span className='text-sm tabular-nums'>
            {round(profile.health)} / {round(max)}
          </span>
        </div>
      )}
      <span className='text-xs font-semibold tracking-wide text-(--mantine-color-dimmed) uppercase'>
        {text('profile.vitals.food', {})}
      </span>
      {profile.food === null ? (
        <Unknown />
      ) : (
        <div className='flex flex-wrap items-center justify-between gap-x-3 gap-y-1'>
          <IconBar
            fills={iconFills(profile.food, 20)}
            icon={faDrumstickBite}
            color='#b5772e'
            label={`${profile.food} / 20`}
          />
          <span className='text-sm tabular-nums'>
            {profile.food} / 20
            {profile.saturation !== null && (
              <span className='ml-2 text-xs text-(--mantine-color-dimmed)'>
                {text('profile.vitals.saturation', { value: round(profile.saturation) })}
              </span>
            )}
          </span>
        </div>
      )}
    </Section>
  );
}

function Experience() {
  const text = useText();
  const { language } = useTranslations();
  const { profile, label, editAccess, act } = useProfileView();
  const [level, setLevel] = useState<number | string>(profile.xp_level ?? 0);
  const [saving, setSaving] = useState(false);
  const progress = Math.min(1, Math.max(0, profile.xp_progress ?? 0));
  const target = typeof level === 'number' ? level : Number.parseInt(level, 10);
  const valid = Number.isInteger(target) && target >= 0 && target <= XP_LEVEL_MAX;

  const save = async () => {
    if (!valid) return;
    setSaving(true);
    await act({ action: 'set_xp_level', level: target }, text('profile.xp.done', { name: label, level: target }));
    setSaving(false);
  };

  return (
    <Section title={text('profile.vitals.experience', {})} icon={faStar}>
      {profile.xp_level === null ? (
        <Unknown />
      ) : (
        <div className='flex flex-col gap-1.5'>
          <span
            className='self-center text-2xl leading-none font-bold tabular-nums'
            style={{ color: '#80ff20', textShadow: '1px 1px 0 #1f3f08, -1px -1px 0 #1f3f08' }}
          >
            {profile.xp_level}
          </span>
          <Progress
            value={progress * 100}
            color='lime'
            size='md'
            radius='xs'
            aria-label={text('profile.vitals.experience', {})}
          />
          <div className='flex flex-wrap justify-between gap-x-3 text-xs text-(--mantine-color-dimmed)'>
            <span>{text('profile.vitals.progress', { percent: Math.floor(progress * 100) })}</span>
            {profile.xp_total !== null && (
              <span>
                {text('profile.vitals.total', { total: new Intl.NumberFormat(language).format(profile.xp_total) })}
              </span>
            )}
          </div>
        </div>
      )}
      {editAccess.visible && (
        <div className='flex items-end gap-2'>
          <NumberInput
            className='flex-1'
            label={text('profile.xp.level', {})}
            min={0}
            max={XP_LEVEL_MAX}
            clampBehavior='strict'
            allowDecimal={false}
            value={level}
            onChange={setLevel}
            disabled={editAccess.blocker !== null}
          />
          <GatedButton
            icon={faStar}
            label={text('profile.xp.set', {})}
            variant='default'
            access={editAccess}
            loading={saving}
            onClick={save}
          />
        </div>
      )}
    </Section>
  );
}

function GameMode() {
  const text = useText();
  const { profile, label, editAccess, act } = useProfileView();
  const [pending, setPending] = useState<Gamemode | null>(null);
  const reason = editAccess.blocker && text(`blockers.${editAccess.blocker}`, {});

  const change = async (gamemode: Gamemode) => {
    setPending(gamemode);
    await act(
      { action: 'set_gamemode', gamemode },
      text('profile.gamemode.set', { name: label, mode: text(`profile.gamemodes.${gamemode}`, {}) }),
    );
    setPending(null);
  };

  if (!editAccess.visible) return null;

  return (
    <Section title={text('profile.gamemode.title', {})} icon={faGamepad}>
      <SegmentedControl
        fullWidth
        value={pending ?? profile.gamemode ?? ''}
        disabled={reason !== null || pending !== null}
        data={GAMEMODES.map((gamemode) => ({ value: gamemode, label: text(`profile.gamemodes.${gamemode}`, {}) }))}
        onChange={(value) => {
          const gamemode = GAMEMODES.find((mode) => mode === value);
          if (gamemode && gamemode !== profile.gamemode) change(gamemode);
        }}
      />
      {reason && <span className='text-xs text-(--mantine-color-dimmed)'>{reason}</span>}
    </Section>
  );
}

function Coordinates({ position }: { position: Position }) {
  const { ui } = usePlayerManager();
  const { t } = useExtTranslations();

  return (
    <div className='flex flex-col gap-1'>
      <span className='font-mono text-sm tabular-nums'>
        {[position.x, position.y, position.z].map((value) => Math.floor(value)).join(' / ')}
      </span>
      <span className='text-xs text-(--mantine-color-dimmed)'>{ui.profile.dimensionLabel(t, position.dimension)}</span>
    </div>
  );
}

function Place() {
  const text = useText();
  const { profile } = useProfileView();

  return (
    <Section title={text('profile.place.position', {})} icon={faLocationDot}>
      {profile.position ? <Coordinates position={profile.position} /> : <Unknown />}
      <span className='inline-flex items-center gap-2 text-xs font-semibold tracking-wide text-(--mantine-color-dimmed) uppercase'>
        <FontAwesomeIcon icon={faBed} />
        {text('profile.place.spawn', {})}
      </span>
      {profile.spawn ? (
        <Coordinates position={profile.spawn} />
      ) : (
        <Tooltip label={text('profile.place.worldSpawnHint', {})} multiline maw={260}>
          <span className='cursor-help text-sm'>{text('profile.place.worldSpawn', {})}</span>
        </Tooltip>
      )}
    </Section>
  );
}

function Effects() {
  const text = useText();
  const { profile } = useProfileView();

  return (
    <Section title={text('profile.effects.title', {})} icon={faFlask}>
      {profile.effects.length === 0 ? (
        <span className='text-sm text-(--mantine-color-dimmed)'>{text('profile.effects.none', {})}</span>
      ) : (
        <ul className='flex flex-col divide-y divide-(--mantine-color-default-border)'>
          {profile.effects.map((effect) => (
            <li key={effect.id} className='flex items-center justify-between gap-3 py-1.5 text-sm'>
              <span>
                {prettifyId(effect.id)} {romanNumeral(effect.amplifier + 1)}
              </span>
              <span className='font-mono text-xs text-(--mantine-color-dimmed) tabular-nums'>
                {formatTicksClock(effect.duration) ?? text('profile.effects.infinite', {})}
              </span>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}

export default function OverviewSection() {
  const { profile } = useProfileView();

  return (
    <div className='grid grid-cols-1 gap-3 md:grid-cols-2'>
      <Vitals />
      {/* a refetched level replaces whatever was typed */}
      <Experience key={profile.xp_level ?? 'none'} />
      <GameMode />
      <Place />
      <Effects />
    </div>
  );
}

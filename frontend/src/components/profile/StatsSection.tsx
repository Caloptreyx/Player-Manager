import {
  faArrowUp,
  faChartColumn,
  faClock,
  faCrosshairs,
  faDove,
  faHandFist,
  faMagnifyingGlass,
  faPersonRunning,
  faPersonWalking,
  faShieldHalved,
  faSkull,
  faSkullCrossbones,
  type IconDefinition,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import ThemeIcon from '@/elements/data-display/ThemeIcon.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import type { HighlightIcon } from '../../games/index.ts';
import { matchesFilter } from '../../lib/players.ts';
import { formatStat } from '../../lib/profiles.ts';
import { useExtTranslations } from '../../translations.ts';
import { usePlayerManager, useText } from '../playerManager.ts';
import ItemIcon from './ItemIcon.tsx';
import { useProfileView } from './profileView.ts';

const HIGHLIGHT_ICONS: Record<HighlightIcon, IconDefinition> = {
  time: faClock,
  death: faSkull,
  mobKill: faCrosshairs,
  playerKill: faSkullCrossbones,
  walk: faPersonWalking,
  run: faPersonRunning,
  fly: faDove,
  jump: faArrowUp,
  attack: faHandFist,
  defense: faShieldHalved,
};

// the key stats as tiles, then one searchable table per category, largest values first
export default function StatsSection() {
  const text = useText();
  const { t } = useExtTranslations();
  const { language } = useTranslations();
  const { ui } = usePlayerManager();
  const { profile } = useProfileView();
  const [picked, setPicked] = useState<string | null>(null);
  const [filter, setFilter] = useState('');

  const { stats } = profile;
  if (!stats || Object.keys(stats).length === 0) {
    return (
      <EmptyState
        flush
        icon={faChartColumn}
        title={text('profile.stats.noneTitle', {})}
        description={text('profile.stats.none', {})}
      />
    );
  }

  const categories = ui.profile.statCategories(t, Object.keys(stats));
  const category = categories.find((entry) => entry.id === picked) ?? categories[0];
  const values = stats[category.id] ?? {};
  const rows = Object.entries(values)
    .map(([key, value]) => ({
      key,
      value,
      label: ui.profile.statLabel(key),
      icon: ui.profile.statIcon(category.id, key),
    }))
    .filter((row) => matchesFilter([row.key, row.label], filter))
    .sort((a, b) => b.value - a.value || a.label.localeCompare(b.label));
  // rows of blocks, items and mobs all get an icon cell (a tile for unknown ones); general stats have none
  const withIcons = rows.some((row) => row.icon !== null);

  return (
    <div className='flex flex-col gap-4'>
      <div className='grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5'>
        {ui.profile.statHighlights(t, stats).map((highlight) => (
          <Card key={highlight.key} padding='sm'>
            <div className='flex items-center gap-2.5'>
              <ThemeIcon size={32} radius='md' variant='light' color='gray'>
                <FontAwesomeIcon icon={HIGHLIGHT_ICONS[highlight.icon]} />
              </ThemeIcon>
              <div className='flex min-w-0 flex-col'>
                <span className='truncate text-xs text-(--mantine-color-dimmed)'>{highlight.label}</span>
                <span className='truncate font-semibold tabular-nums'>
                  {formatStat(highlight.format, highlight.value, language)}
                </span>
              </div>
            </div>
          </Card>
        ))}
      </div>

      <div className='flex flex-wrap gap-1.5'>
        {categories.map((entry) => (
          <Button
            key={entry.id}
            size='xs'
            radius='xl'
            variant={entry.id === category.id ? 'light' : 'default'}
            onClick={() => setPicked(entry.id)}
            rightSection={
              <span className='text-xs text-(--mantine-color-dimmed) tabular-nums'>
                {Object.keys(stats[entry.id] ?? {}).length}
              </span>
            }
          >
            {entry.label}
          </Button>
        ))}
      </div>

      <TextInput
        className='w-full sm:w-72'
        placeholder={text('profile.stats.search', {})}
        aria-label={text('profile.stats.search', {})}
        leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} size='sm' />}
        value={filter}
        onChange={(e) => setFilter(e.currentTarget.value)}
      />

      {rows.length === 0 ? (
        <p className='py-6 text-center text-sm text-(--mantine-color-dimmed)'>
          {text('lists.noMatches', { query: filter.trim() })}
        </p>
      ) : (
        <Table
          columns={[
            text('profile.stats.name', {}),
            { name: text('profile.stats.value', {}), className: 'text-right [&>div]:justify-end' },
          ]}
        >
          {rows.map((row) => (
            <TableRow key={row.key}>
              <TableData>
                <span className='flex min-w-0 items-center gap-2.5'>
                  {withIcons && (
                    <span className='grid size-7 shrink-0 place-items-center rounded-sm bg-[#8b8b8b]'>
                      <ItemIcon id={row.key} src={row.icon} size={22} />
                    </span>
                  )}
                  <span className='flex min-w-0 flex-col'>
                    <span className='truncate text-sm'>{row.label}</span>
                    <span className='truncate font-mono text-xs text-(--mantine-color-dimmed)'>{row.key}</span>
                  </span>
                </span>
              </TableData>
              <TableData className='w-px text-right font-medium tabular-nums'>
                {formatStat(ui.profile.statFormat(category.id, row.key), row.value, language)}
              </TableData>
            </TableRow>
          ))}
        </Table>
      )}
    </div>
  );
}

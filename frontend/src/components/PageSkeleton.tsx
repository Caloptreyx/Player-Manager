import { Skeleton } from '@mantine/core';
import Card from '@/elements/data-display/Card.tsx';
import { TilesSkeleton } from './StatTiles.tsx';

// the shape of the page while the overview loads: header, tiles and a few table rows
export default function PageSkeleton() {
  return (
    <div className='flex flex-col gap-4'>
      <Card className='flex flex-row! items-center gap-3'>
        <Skeleton height={42} width={42} radius='md' />
        <div className='flex flex-1 flex-col gap-2'>
          <Skeleton height={14} width={160} />
          <Skeleton height={10} width={240} />
        </div>
        <Skeleton height={24} width={120} radius='xl' />
      </Card>
      <TilesSkeleton />
      <Card className='flex flex-col gap-4'>
        <div className='flex gap-4'>
          {[80, 96, 90, 70].map((width) => (
            <Skeleton key={width} height={14} width={width} />
          ))}
        </div>
        <Skeleton height={34} width={288} />
        {[0, 1, 2, 3].map((row) => (
          <div key={row} className='flex items-center gap-3'>
            <Skeleton height={32} width={32} radius='md' />
            <div className='flex flex-1 flex-col gap-1.5'>
              <Skeleton height={12} width='20%' />
              <Skeleton height={10} width='35%' />
            </div>
          </div>
        ))}
      </Card>
    </div>
  );
}

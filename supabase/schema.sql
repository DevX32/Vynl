create table if not exists public.playlists (
  user_id uuid not null references auth.users (id) on delete cascade,
  local_id text not null,
  name text not null,
  tracks jsonb not null default '[]'::jsonb,
  adds jsonb not null default '[]'::jsonb,
  removals jsonb not null default '[]'::jsonb,
  created_at bigint not null,
  updated_at bigint not null,
  deleted_at bigint,
  primary key (user_id, local_id)
);

alter table public.playlists
  add column if not exists adds jsonb not null default '[]'::jsonb;

alter table public.playlists
  add column if not exists removals jsonb not null default '[]'::jsonb;

alter table public.playlists enable row level security;

drop policy if exists "own playlists" on public.playlists;
create policy "own playlists" on public.playlists
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

create table if not exists public.library_order (
  user_id uuid primary key references auth.users (id) on delete cascade,
  order_ids jsonb not null default '[]'::jsonb,
  updated_at bigint not null
);

alter table public.library_order enable row level security;

drop policy if exists "own order" on public.library_order;
create policy "own order" on public.library_order
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

create index if not exists playlists_user_updated
  on public.playlists (user_id, updated_at desc);

create table if not exists public.backups (
  user_id uuid primary key references auth.users (id) on delete cascade,
  data jsonb not null default '{}'::jsonb,
  updated_at bigint not null
);

alter table public.backups enable row level security;

drop policy if exists "own backups" on public.backups;
create policy "own backups" on public.backups
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);
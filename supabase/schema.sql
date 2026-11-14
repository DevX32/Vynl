create table if not exists public.playlists (
  user_id uuid not null references auth.users (id) on delete cascade,
  local_id text not null,
  name text not null,
  tracks jsonb not null default '[]'::jsonb,
  created_at bigint not null,
  updated_at bigint not null,
  deleted_at bigint,
  primary key (user_id, local_id)
);

alter table public.playlists enable row level security;

drop policy if exists "own playlists" on public.playlists;
create policy "own playlists" on public.playlists
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

create index if not exists playlists_user_updated
  on public.playlists (user_id, updated_at desc);

create table if not exists public.sync_state (
  user_id uuid primary key references auth.users (id) on delete cascade,
  data jsonb not null default '{}'::jsonb,
  updated_at bigint not null
);

alter table public.sync_state enable row level security;

drop policy if exists "own sync state" on public.sync_state;
create policy "own sync state" on public.sync_state
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

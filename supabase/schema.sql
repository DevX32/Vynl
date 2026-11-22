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
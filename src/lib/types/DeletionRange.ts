export type DeletionRange =
  | { type: 'days'; value: number }
  | { type: 'months'; value: number }
  | { type: 'all' }

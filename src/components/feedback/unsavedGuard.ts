import type { Composer } from 'vue-i18n'

type Translate = Composer['t']

export type UnsavedDialogApi = {
  warning: (options: {
    title: string
    content: string
    positiveText: string
    negativeText: string
    onPositiveClick?: () => void | boolean | Promise<void | boolean>
    onNegativeClick?: () => void | boolean | Promise<void | boolean>
    onClose?: () => void
  }) => void
}

/**
 * Ask the user to discard unsaved work when `dirty` is true.
 * Returns true when navigation / mode switch may proceed.
 */
export async function confirmDiscardIfNeeded(
  dirty: boolean,
  dialog: UnsavedDialogApi,
  t: Translate,
): Promise<boolean> {
  if (!dirty) return true

  return new Promise((resolve) => {
    let settled = false
    const finish = (value: boolean) => {
      if (settled) return
      settled = true
      resolve(value)
    }

    dialog.warning({
      title: t('editor.unsavedTitle'),
      content: t('editor.unsavedContent'),
      positiveText: t('editor.discard'),
      negativeText: t('forms.cancel'),
      onPositiveClick: () => {
        finish(true)
      },
      onNegativeClick: () => {
        finish(false)
      },
      onClose: () => {
        finish(false)
      },
    })
  })
}

/**
 * Confirm discarding unapplied source-mode drafts when leaving source mode.
 */
export async function confirmDiscardSourceIfNeeded(
  sourceDirty: boolean,
  dialog: UnsavedDialogApi,
  t: Translate,
): Promise<boolean> {
  if (!sourceDirty) return true

  return new Promise((resolve) => {
    let settled = false
    const finish = (value: boolean) => {
      if (settled) return
      settled = true
      resolve(value)
    }

    dialog.warning({
      title: t('editor.discardSourceTitle'),
      content: t('editor.discardSourceContent'),
      positiveText: t('editor.discard'),
      negativeText: t('forms.cancel'),
      onPositiveClick: () => {
        finish(true)
      },
      onNegativeClick: () => {
        finish(false)
      },
      onClose: () => {
        finish(false)
      },
    })
  })
}

/**
 * Factory for Vue Router `onBeforeRouteLeave` / `beforeRouteLeave`.
 * Pass a getter so dirty is evaluated at leave time.
 */
export function unsavedGuard(
  isDirty: () => boolean,
  dialog: UnsavedDialogApi,
  t: Translate,
): () => Promise<boolean> {
  return () => confirmDiscardIfNeeded(isDirty(), dialog, t)
}

/** Native beforeunload hook; returns a disposer. */
export function attachBeforeUnload(isDirty: () => boolean): () => void {
  const handler = (event: BeforeUnloadEvent) => {
    if (!isDirty()) return
    event.preventDefault()
    event.returnValue = ''
  }
  window.addEventListener('beforeunload', handler)
  return () => window.removeEventListener('beforeunload', handler)
}

'use client';

import { useEffect, useRef } from 'react';
import { AlertTriangle, X } from 'lucide-react';

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  countdown?: number;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = '확인',
  cancelLabel = '취소',
  danger = false,
  countdown,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    if (open) {
      dialogRef.current?.showModal();
    } else {
      dialogRef.current?.close();
    }
  }, [open]);

  if (!open) return null;

  return (
    <dialog
      ref={dialogRef}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClose={onCancel}
    >
      <div className="w-96 rounded-lg bg-bg-secondary border border-gray-700 p-6 shadow-xl">
        <div className="flex items-start gap-3">
          {danger && (
            <AlertTriangle className="mt-0.5 text-red-400 shrink-0" size={20} />
          )}
          <div className="flex-1">
            <h3 className="text-lg font-semibold text-white">{title}</h3>
            <p className="mt-2 text-sm text-gray-400">{message}</p>
          </div>
          <button onClick={onCancel} className="text-gray-500 hover:text-white">
            <X size={18} />
          </button>
        </div>

        <div className="mt-6 flex justify-end gap-3">
          <button
            onClick={onCancel}
            className="rounded-md bg-bg-tertiary px-4 py-2 text-sm text-gray-300 hover:bg-gray-600"
          >
            {cancelLabel}
          </button>
          <button
            onClick={onConfirm}
            className={`rounded-md px-4 py-2 text-sm font-semibold text-white ${
              danger
                ? 'bg-red-600 hover:bg-red-700'
                : 'bg-accent-blue hover:bg-blue-600'
            }`}
          >
            {confirmLabel}
            {countdown !== undefined && countdown > 0 && ` (${countdown})`}
          </button>
        </div>
      </div>
    </dialog>
  );
}

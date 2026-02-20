import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export interface RetranscriptionProgress {
  meeting_id: string;
  stage: string;
  progress_percentage: number;
  message: string;
}

export interface RetranscriptionResult {
  meeting_id: string;
  segments_count: number;
  duration_seconds: number;
  language: string | null;
}

export interface RetranscriptionError {
  meeting_id: string;
  error: string;
}

export type RetranscriptionStatus = 'idle' | 'processing' | 'complete' | 'error';

export interface UseRetranscriptionOptions {
  meetingId: string;
  onComplete?: (result: RetranscriptionResult) => void;
  onError?: (error: string) => void;
}

export interface UseRetranscriptionReturn {
  status: RetranscriptionStatus;
  progress: RetranscriptionProgress | null;
  error: string | null;
  isProcessing: boolean;
  startRetranscription: (folderPath: string, language?: string | null, model?: string | null, provider?: string | null) => Promise<void>;
  cancelRetranscription: () => Promise<void>;
  reset: () => void;
}

export function useRetranscription({
  meetingId,
  onComplete,
  onError,
}: UseRetranscriptionOptions): UseRetranscriptionReturn {
  const [status, setStatus] = useState<RetranscriptionStatus>('idle');
  const [progress, setProgress] = useState<RetranscriptionProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Stable refs for callbacks to avoid listener re-registration
  const onCompleteRef = useRef(onComplete);
  const onErrorRef = useRef(onError);
  useEffect(() => { onCompleteRef.current = onComplete; }, [onComplete]);
  useEffect(() => { onErrorRef.current = onError; }, [onError]);

  // Set up event listeners (re-register only when meetingId changes)
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    const setupListeners = async () => {
      // Progress events
      const unlistenProgress = await listen<RetranscriptionProgress>(
        'retranscription-progress',
        (event) => {
          if (event.payload.meeting_id === meetingId) {
            setProgress(event.payload);
            setStatus('processing');
          }
        }
      );
      unlisteners.push(unlistenProgress);

      // Completion event
      const unlistenComplete = await listen<RetranscriptionResult>(
        'retranscription-complete',
        (event) => {
          if (event.payload.meeting_id === meetingId) {
            setStatus('complete');
            setProgress(null);
            onCompleteRef.current?.(event.payload);
          }
        }
      );
      unlisteners.push(unlistenComplete);

      // Error event
      const unlistenError = await listen<RetranscriptionError>(
        'retranscription-error',
        (event) => {
          if (event.payload.meeting_id === meetingId) {
            setStatus('error');
            setError(event.payload.error);
            onErrorRef.current?.(event.payload.error);
          }
        }
      );
      unlisteners.push(unlistenError);
    };

    setupListeners();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [meetingId]);

  const startRetranscription = useCallback(
    async (folderPath: string, language?: string | null, model?: string | null, provider?: string | null) => {
      setStatus('processing');
      setError(null);
      setProgress(null);

      try {
        await invoke('start_retranscription_command', {
          meetingId,
          meetingFolderPath: folderPath,
          language: language || null,
          model: model || null,
          provider: provider || null,
        });
      } catch (err: any) {
        setStatus('error');
        setError(err.message || 'Failed to start retranscription');
        onErrorRef.current?.(err.message || 'Failed to start retranscription');
      }
    },
    [meetingId]
  );

  const cancelRetranscription = useCallback(async () => {
    try {
      await invoke('cancel_retranscription_command');
      setStatus('idle');
      setProgress(null);
    } catch (err: any) {
      console.error('Failed to cancel retranscription:', err);
    }
  }, []);

  const reset = useCallback(() => {
    setStatus('idle');
    setProgress(null);
    setError(null);
  }, []);

  return {
    status,
    progress,
    error,
    isProcessing: status === 'processing',
    startRetranscription,
    cancelRetranscription,
    reset,
  };
}

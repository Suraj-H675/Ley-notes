import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle, RotateCcw } from 'lucide-react';

interface Props { children: ReactNode; feature: string; resetKey?: string | number | null; overlay?: boolean }
interface State { error: Error | null }

export class FeatureErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State { return { error }; }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`[${this.props.feature}] feature failed`, error, info.componentStack);
  }

  componentDidUpdate(previous: Props) {
    if (this.state.error && previous.resetKey !== this.props.resetKey) this.setState({ error: null });
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className={this.props.overlay ? 'fixed inset-0 z-[100] flex items-center justify-center bg-background p-6' : 'flex h-full min-h-60 items-center justify-center bg-background p-6'} role="alert">
        <div className="max-w-md rounded-md border border-destructive/30 bg-surface-1 p-6 text-center shadow-panel">
          <AlertTriangle size={24} className="mx-auto text-destructive" />
          <h2 className="mt-3 text-body font-semibold text-foreground">{this.props.feature} could not open</h2>
          <p className="mt-1 text-meta leading-relaxed text-muted-foreground">Your vault data is unchanged. Retry the feature; if it fails again, reload Ley to rebuild its local UI state.</p>
          <button type="button" onClick={() => this.setState({ error: null })} className="mx-auto mt-4 flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-meta font-medium text-primary-foreground"><RotateCcw size={13} />Try again</button>
        </div>
      </div>
    );
  }
}

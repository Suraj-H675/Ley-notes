import {
  format,
  formatDistanceToNow,
  isToday,
  isYesterday,
  isThisWeek,
  isThisYear,
  parseISO,
} from 'date-fns';

export function formatDate(timestamp: number): string {
  const date = new Date(timestamp);

  if (isToday(date)) {
    return format(date, "'Today at' h:mm a");
  }

  if (isYesterday(date)) {
    return format(date, "'Yesterday at' h:mm a");
  }

  if (isThisWeek(date)) {
    return format(date, "EEEE 'at' h:mm a");
  }

  if (isThisYear(date)) {
    return format(date, "MMM d 'at' h:mm a");
  }

  return format(date, 'MMM d, yyyy');
}

export function formatDateShort(timestamp: number): string {
  const date = new Date(timestamp);

  if (isToday(date)) {
    return format(date, 'h:mm a');
  }

  if (isYesterday(date)) {
    return "'Yesterday'";
  }

  if (isThisWeek(date)) {
    return format(date, 'EEE');
  }

  if (isThisYear(date)) {
    return format(date, 'MMM d');
  }

  return format(date, 'MMM d, yyyy');
}

export function formatRelative(timestamp: number): string {
  return formatDistanceToNow(new Date(timestamp), { addSuffix: true });
}

export function parseDate(dateString: string): Date | null {
  try {
    return parseISO(dateString);
  } catch {
    return null;
  }
}

export function isOverdue(dueDate: number): boolean {
  return dueDate < Date.now();
}

export function isDueSoon(dueDate: number, daysThreshold = 3): boolean {
  const threshold = daysThreshold * 24 * 60 * 60 * 1000;
  return dueDate > Date.now() && dueDate < Date.now() + threshold;
}

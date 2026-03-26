import { h } from 'https://esm.sh/preact@10';
import { useState } from 'https://esm.sh/preact@10/hooks';
import htm from 'https://esm.sh/htm@3';

const html = htm.bind(h);

const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTH_NAMES = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December'
];

function formatDate(year, month, day) {
  return `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

function getDaysInMonth(year, month) {
  return new Date(year, month + 1, 0).getDate();
}

function getFirstDayOfWeek(year, month) {
  return new Date(year, month, 1).getDay();
}

function todayStr() {
  const d = new Date();
  return formatDate(d.getFullYear(), d.getMonth(), d.getDate());
}

export function PopulateCalendar({ populatedDates, selectedStart, selectedEnd, onSelectDate, disabled }) {
  const [viewDate, setViewDate] = useState(() => {
    if (selectedStart) {
      const parts = selectedStart.split('-');
      return new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, 1);
    }
    return new Date();
  });

  const today = todayStr();
  const year = viewDate.getFullYear();
  const month = viewDate.getMonth();
  const daysInMonth = getDaysInMonth(year, month);
  const firstDay = getFirstDayOfWeek(year, month);

  // Previous month fill
  const prevMonth = month === 0 ? 11 : month - 1;
  const prevYear = month === 0 ? year - 1 : year;
  const daysInPrevMonth = getDaysInMonth(prevYear, prevMonth);

  function prevMonthClick() {
    setViewDate(new Date(year, month - 1, 1));
  }

  function nextMonthClick() {
    setViewDate(new Date(year, month + 1, 1));
  }

  function isInRange(dateStr) {
    if (!selectedStart || !selectedEnd) return false;
    return dateStr >= selectedStart && dateStr <= selectedEnd;
  }

  function isRangeEnd(dateStr) {
    return dateStr === selectedStart || dateStr === selectedEnd;
  }

  function isPopulated(dateStr) {
    return populatedDates && populatedDates.has(dateStr);
  }

  function handleDayClick(dateStr, outside) {
    if (disabled || outside || !onSelectDate) return;
    onSelectDate(dateStr);
  }

  // Build day cells
  const cells = [];

  // Previous month trailing days
  for (let i = firstDay - 1; i >= 0; i--) {
    const day = daysInPrevMonth - i;
    const dateStr = formatDate(prevYear, prevMonth, day);
    cells.push({ day, dateStr, outside: true });
  }

  // Current month days
  for (let d = 1; d <= daysInMonth; d++) {
    const dateStr = formatDate(year, month, d);
    cells.push({ day: d, dateStr, outside: false });
  }

  // Next month leading days (fill to complete last row)
  const remaining = 7 - (cells.length % 7);
  if (remaining < 7) {
    const nextMo = month === 11 ? 0 : month + 1;
    const nextYr = month === 11 ? year + 1 : year;
    for (let d = 1; d <= remaining; d++) {
      const dateStr = formatDate(nextYr, nextMo, d);
      cells.push({ day: d, dateStr, outside: true });
    }
  }

  return html`
    <div class="populate-calendar">
      <div class="calendar-nav">
        <button class="outline calendar-nav-btn" onclick=${prevMonthClick} type="button" aria-label="Previous month">
          \u25C0
        </button>
        <strong>${MONTH_NAMES[month]} ${year}</strong>
        <button class="outline calendar-nav-btn" onclick=${nextMonthClick} type="button" aria-label="Next month">
          \u25B6
        </button>
      </div>
      <div class="calendar-grid">
        ${DAY_NAMES.map(d => html`<div class="calendar-header">${d}</div>`)}
        ${cells.map(c => {
          const populated = isPopulated(c.dateStr);
          const inRange = isInRange(c.dateStr);
          const rangeEnd = isRangeEnd(c.dateStr);
          const conflict = populated && inRange;
          const isToday = c.dateStr === today;
          let cls = 'calendar-day';
          if (c.outside) cls += ' calendar-day--outside';
          else if (conflict) cls += ' calendar-day--conflict';
          else if (rangeEnd) cls += ' calendar-day--range-end';
          else if (populated) cls += ' calendar-day--populated';
          else if (inRange) cls += ' calendar-day--selected';
          if (isToday && !c.outside) cls += ' calendar-day--today';
          if (!c.outside && !disabled && onSelectDate) cls += ' calendar-day--clickable';
          return html`<div class=${cls}
                           onclick=${() => handleDayClick(c.dateStr, c.outside)}>${c.day}</div>`;
        })}
      </div>
    </div>
  `;
}

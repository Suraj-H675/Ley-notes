import { Routes, Route } from 'react-router-dom';
import { AppShell } from '@/components/layout';
import { CommandPalette } from '@/components/search/CommandPalette';
import { HomePage } from '@/pages/HomePage';
import { DocumentPage } from '@/pages/DocumentPage';
import { UniversePage } from '@/pages/UniversePage';
import { TasksPage } from '@/pages/TasksPage';
import { ProjectsPage } from '@/pages/ProjectsPage';
import { CollectionsPage } from '@/pages/CollectionsPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { NotFoundPage } from '@/pages/NotFoundPage';

function App() {
  return (
    <>
      <CommandPalette />
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/page/:id" element={<DocumentPage />} />
          <Route path="/universe" element={<UniversePage />} />
          <Route path="/tasks" element={<TasksPage />} />
          <Route path="/projects" element={<ProjectsPage />} />
          <Route path="/collections" element={<CollectionsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<NotFoundPage />} />
        </Route>
      </Routes>
    </>
  );
}

export default App;

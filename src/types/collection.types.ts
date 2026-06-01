export interface Collection {
  id: string;
  name: string;
  emoji?: string;
  parentId?: string;
  createdAt: number;
  updatedAt: number;
}

export interface CreateCollectionInput {
  name: string;
  emoji?: string;
  parentId?: string;
}

export interface UpdateCollectionInput {
  name?: string;
  emoji?: string;
  parentId?: string;
}

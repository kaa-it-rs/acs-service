// mongodb://openers:Q123456q@127.0.0.1:27017/admin?authSource=openers

let now = new Date();

db.createUser({
  user: "openers",
  pwd: "Q123456q",
  roles: [
    {
      role: "readWrite",
      db: "openers",
    },
  ],
});

db.createCollection("openers");
db.openers.createIndex(
  { serialNumber: 1 },
  { name: "serialNumber", unique: true }
);

db.createCollection("roles");
db.roles.insert({
  name: "admin",
  accessRights: {
    users: {
      list: true,
      view: true,
      create: true,
      edit: true,
      delete: true,
    },
    roles: {
      list: true,
      view: true,
      create: true,
      edit: true,
      delete: true,
    },
    openers: {
      list: true,
      view: true,
      create: true,
      edit: true,
      delete: true,
    },
    barrierManufacturers: {
      list: true,
      view: true,
      create: true,
      edit: true,
      delete: true,
    },
    barrierModels: {
      list: true,
      view: true,
      create: true,
      edit: true,
      delete: true,
    }
  },
  createdAt: now,
  updatedAt: now,
});
db.roles.insert({
  name: "manufacturer",
  accessRights: {
    users: {
      list: false,
      view: false,
      create: false,
      edit: false,
      delete: false,
    },
    roles: {
      list: false,
      view: false,
      create: false,
      edit: false,
      delete: false,
    },
    openers: {
      list: false,
      view: false,
      create: true,
      edit: false,
      delete: false,
    },
    barrierManufacturers: {
      list: false,
      view: false,
      create: false,
      edit: false,
      delete: false,
    },
    barrierModels: {
      list: false,
      view: false,
      create: false,
      edit: false,
      delete: false,
    }
  },
  createdAt: now,
  updatedAt: now,
});
db.roles.insert({
  name: "normal",
  accessRights: {
    users: {
      list: false,
      view: false,
      create: false,
      edit: false,
      delete: false,
    },
    roles: {
      list: false,
      view: false,
      create: false,
      edit: false,
      delete: false,
    },
    openers: {
      list: true,
      view: true,
      create: false,
      edit: true,
      delete: false,
    },
    barrierManufacturers: {
      list: true,
      view: true,
      create: false,
      edit: false,
      delete: false,
    },
    barrierModels: {
      list: true,
      view: true,
      create: false,
      edit: false,
      delete: false,
    }
  },
  createdAt: now,
  updatedAt: now,
});

db.createCollection("users");
db.users.createIndex({ login: 1 }, { name: "login", unique: true });
db.users.insert({
  login: "localadmin",
  password: "$2a$04$HgLuKmwaOzo6U81YPKnt/uVJXZCAYtZLFYLBI.7XlySLT7P/zLf5O",
  roleId: db.roles.findOne({ name: "admin" })["_id"],
  createdAt: now,
  updatedAt: now,
});
db.users.insert({
  login: "manufacturer",
  password: "$2a$04$/ZdiGm3P7.j4xUTn4RwQwOfUArRBLgpjMTlcIVLI16NOXjLzd7wba",
  roleId: db.roles.findOne({ name: "manufacturer" })["_id"],
  createdAt: now,
  updatedAt: now,
});
db.users.insert({
  login: "vasya",
  password: "$2b$04$qm2TWn6N3SFOLat1fXvFmuhaGmQJvaAhMw1vzfWE3oTOx5ed0AEWy",
  roleId: db.roles.findOne({ name: "normal" })["_id"],
  createdAt: now,
  updatedAt: now,
});

db.createCollection("clients");
db.clients.createIndex(
  { refreshToken: 1 },
  { name: "refreshToken", unique: true }
);


db.createCollection("barrierManufacturers");
db.openers.createIndex(
  { name: 1 },
  { name: "name", unique: true }
);

db.createCollection("barrierModels");
db.openers.createIndex(
  { name: 1 },
  { name: "name", unique: true }
);
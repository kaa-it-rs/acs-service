// mongodb://openers:Q123456q@127.0.0.1:27017/admin?authSource=openers

// Scheme

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
db.roles.createIndex(
  { name: 1 },
  { name: "name", unique: true }
);

db.createCollection("users");
db.users.createIndex(
  { login: 1 },
  { name: "login", unique: true }
);

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

// Data

let now = new Date();

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

db.users.insert({
  login: "localadmin",
  password: "$2a$04$HgLuKmwaOzo6U81YPKnt/uVJXZCAYtZLFYLBI.7XlySLT7P/zLf5O", // 1QaZ2WsX
  roleId: db.roles.findOne({ name: "admin" })["_id"],
  createdAt: now,
  updatedAt: now,
});

db.users.insert({
  login: "manufacturer",
  password: "$2a$04$e/ppod6d6oQtbKf25J0GSO9dw49Iddola8M6MyS.TYtcAEjBAmx2C", // 123321
  roleId: db.roles.findOne({ name: "manufacturer" })["_id"],
  createdAt: now,
  updatedAt: now,
});

db.users.insert({
  login: "vasya",
  password: "$2a$04$U/TsFOoRLibYXepiyy5Ehe/0ZDN1NjJetYP7EDXZqwtsVYNWrRXXu", // 123456
  roleId: db.roles.findOne({ name: "normal" })["_id"],
  createdAt: now,
  updatedAt: now,
});

db.openers.insert({
  serialNumber: "111",
  connected: false,
  login: "admin",
  password: "admin",
  createdAt: now,
  updatedAt: now,
  version: "1.0.2",
  nonce: "jdfjksdhfjshfkjsdhkfhk",
  commandStatus: "READY"
});

db.barrierManufacturers.insert({
  name: "PERCo",
  createdAt: now,
  updatedAt: now,
});

db.barrierManufacturers.insert({
  name: "Doorhan",
  createdAt: now,
  updatedAt: now,
});

db.barrierManufacturers.insert({
  name: "Came",
  createdAt: now,
  updatedAt: now,
});

db.barrierModels.insert({
  name: "GS04",
  algorithm: "OPEN_CLOSE",
  createdAt: now,
  updatedAt: now,
});

db.barrierModels.insert({
  name: "Barrier PRO 3000",
  algorithm: "OPEN",
  createdAt: now,
  updatedAt: now,
});

db.barrierModels.insert({
  name: "Gard 4040",
  algorithm: "TWO_DOORS",
  createdAt: now,
  updatedAt: now,
});

let perco = db.barrierManufacturers.findOne({name: "PERCo"});
let doorhan = db.barrierManufacturers.findOne({name: "Doorhan"});
let came = db.barrierManufacturers.findOne({name: "Came"});

db.barrierModels.updateOne({name: "GS04"}, {$set: {manufacturerId: perco["_id"]}});
db.barrierModels.updateOne({name: "Barrier PRO 3000"}, {$set: {manufacturerId: doorhan["_id"]}});
db.barrierModels.updateOne({name: "Gard 4040"}, {$set: {manufacturerId: came["_id"]}});

let gs04 = db.barrierModels.findOne({name: "GS04"});
let barrierPro3000 = db.barrierModels.findOne({name: "Barrier PRO 3000"});
let gard4040 = db.barrierModels.findOne({name: "Gard 4040"});

db.barrierManufacturers.updateOne({name: "PERCo"}, {$set: {modelIds: [gs04["_id"]]}});
db.barrierManufacturers.updateOne({name: "Doorhan"}, {$set: {modelIds: [barrierPro3000["_id"]]}});
db.barrierManufacturers.updateOne({name: "Came"}, {$set: {modelIds: [gard4040["_id"]]}});

//db.openers.updateOne({serialNumber: "111"}, {$set: {barrierModelId: gs04["_id"]}});

let vasya = db.users.findOne({login: "vasya"});

db.openers.updateOne({serialNumber: "111"}, {$set: {userId: vasya["_id"]}});

import {DataTypes, Sequelize} from "sequelize";

export const sequelize = new Sequelize({
    dialect: "sqlite",
    storage: "balances.sqlite",
    logging: false,
});

export const Account = sequelize.define(
    "account",
    {
        address: DataTypes.STRING,
        amount: DataTypes.BIGINT,
        free: DataTypes.BIGINT,
        miscFrozen: DataTypes.BIGINT,
        toSlash: DataTypes.BIGINT,
        is_slashed: {
            type: DataTypes.BOOLEAN,
            defaultValue: false,
        },
    }
);
